#!/usr/bin/env python3
"""Multi-copy upload test: upload a random file via filecoin-pin against the devnet."""

from __future__ import annotations

import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from urllib.error import URLError
from urllib.request import urlopen

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from scenarios.dependencies import component
from scenarios.helpers import (
    assert_eq,
    assert_ok,
    fail,
    info,
    run_cmd,
    write_random_file,
)

ANSI_RE = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")
RAND_FILE_NAME = "random_file"
RAND_FILE_SIZE = 20 * 1024 * 1024
RAND_FILE_SEED = 42
ADD_DEADLINE_SECS = 240
ADD_INTERVAL_SECS = 10
ADD_ATTEMPT_TIMEOUT_SECS = 180
RETRIEVAL_DEADLINE_SECS = 90
RETRIEVAL_INTERVAL_SECS = 5
RETRIEVAL_REQUEST_TIMEOUT_SECS = 10
VERIFY_CID_SCRIPT = """
import { readFileSync } from "node:fs";
import { CID } from "multiformats";
import { sha256 } from "multiformats/hashes/sha2";

const [cidString, filePath] = process.argv.slice(1);

if (!cidString || !filePath) {
  console.error("usage: node --input-type=module --eval <script> <cid> <file>");
  process.exit(1);
}

const cid = CID.parse(cidString);
const bytes = readFileSync(filePath);
const digest = await sha256.digest(bytes);

if (Buffer.from(digest.digest).equals(Buffer.from(cid.multihash.digest))) {
  console.log("OK");
} else {
  console.error("CID mismatch");
  process.exit(1);
}
""".strip()


def strip_ansi(value: str) -> str:
    return ANSI_RE.sub("", value)


def output_text(value) -> str:
    if isinstance(value, bytes):
        return value.decode(errors="replace")
    return value or ""


def run_filecoin_pin_add(
    filecoin_pin_command: list[str],
    upload_dir: Path,
    extra_args: list[str],
    label: str,
):
    cmd = [
        *filecoin_pin_command,
        "add",
        "--network",
        "devnet",
        *extra_args,
        str(upload_dir),
    ]
    deadline = time.time() + ADD_DEADLINE_SECS
    attempt = 0
    last_result = None

    while time.time() < deadline:
        attempt += 1
        info(f"filecoin-pin add attempt {attempt} ({label})")

        try:
            result = subprocess.run(
                cmd,
                text=True,
                capture_output=True,
                timeout=ADD_ATTEMPT_TIMEOUT_SECS,
            )
        except subprocess.TimeoutExpired as e:
            stdout = e.stdout or ""
            stderr = e.stderr or ""
            result = subprocess.CompletedProcess(cmd, -1, stdout, stderr)

        last_result = result
        if result.returncode == 0:
            return result

        add_stdout = output_text(result.stdout).strip()
        add_stderr = output_text(result.stderr).strip()
        add_details = strip_ansi(f"{add_stdout}\n{add_stderr}".strip())
        info(
            "filecoin-pin add failed; retrying if time remains "
            f"(label={label}, attempt={attempt}, exit={result.returncode})"
        )
        if add_details:
            info(add_details[-1200:])

        time.sleep(ADD_INTERVAL_SECS)

    return last_result


def setup_filecoin_pin(work_dir: Path) -> tuple[list[str], Path]:
    dependency = component("filecoin-pin")
    source = dependency["source"]
    overrides = dependency.get("overrides", {})

    if source == "npm":
        run_cmd(["npm", "init", "-y"], label="npm init", cwd=work_dir)
        package_fields = [
            "type=module",
            f"dependencies.filecoin-pin={dependency['version']}",
        ]
        for package, spec in sorted(overrides.items()):
            info(f"npm override {package}={spec['version']} ({spec['reason']})")
            package_fields.extend(
                [
                    f"dependencies.{package}={spec['version']}",
                    f"overrides.{package}={spec['version']}",
                ]
            )
        run_cmd(
            ["npm", "pkg", "set", *package_fields],
            label=f"pin filecoin-pin {dependency['version']}",
            cwd=work_dir,
        )
        run_cmd(["npm", "install"], label="npm install", cwd=work_dir)
        return [str(work_dir / "node_modules" / ".bin" / "filecoin-pin")], work_dir

    if source == "git":
        repository_dir = work_dir / "filecoin-pin"
        run_cmd(
            ["git", "clone", dependency["repository"], str(repository_dir)],
            label="clone filecoin-pin",
        )
        run_cmd(
            ["git", "checkout", "--detach", dependency["commit"]],
            cwd=repository_dir,
            label=f"checkout filecoin-pin {dependency['commit']}",
        )
        # Only filecoin-pin itself is needed; skip the upload-action workspace.
        run_cmd(
            ["pnpm", "install", "--frozen-lockfile", "--filter", "filecoin-pin..."],
            cwd=repository_dir,
            label="pnpm install",
        )
        run_cmd(["pnpm", "build"], cwd=repository_dir, label="pnpm build")
        return ["node", str(repository_dir / "dist" / "cli.js")], repository_dir

    raise RuntimeError(f"Unsupported filecoin-pin source: {source}")


def download_and_verify(
    url: str, file: Path, root_cid: str, npm_dir: Path
) -> str | None:
    download_url = f"{url}?format=raw"
    last_error = ""
    deadline = time.time() + RETRIEVAL_DEADLINE_SECS

    while time.time() < deadline:
        file.unlink(missing_ok=True)

        try:
            with urlopen(
                download_url, timeout=RETRIEVAL_REQUEST_TIMEOUT_SECS
            ) as resp, open(file, "wb") as f:
                while True:
                    chunk = resp.read(1024 * 1024)
                    if not chunk:
                        break
                    f.write(chunk)
        except (URLError, OSError) as e:
            last_error = f"Failed to download {download_url}: {e}"
        else:
            verify_result = subprocess.run(
                [
                    "node",
                    "--input-type=module",
                    "--eval",
                    VERIFY_CID_SCRIPT,
                    root_cid,
                    str(file.resolve()),
                ],
                cwd=npm_dir,
                text=True,
                capture_output=True,
            )

            if verify_result.returncode == 0:
                return None

            verify_details = (
                verify_result.stderr or verify_result.stdout or ""
            ).strip()
            last_error = (
                f"CID verification failed for {download_url} "
                f"(exit={verify_result.returncode}) {verify_details}"
            )

        time.sleep(RETRIEVAL_INTERVAL_SECS)

    return last_error or f"Timed out verifying {download_url}"


def run():
    assert_ok("command -v node", "node is installed")
    assert_ok("command -v npm", "npm is installed")

    with tempfile.TemporaryDirectory(
        prefix="filecoin-pin-upload-"
    ) as upload_tmp, tempfile.TemporaryDirectory(
        prefix="filecoin-pin-npm-"
    ) as npm_tmp, tempfile.TemporaryDirectory(
        prefix="filecoin-pin-download-"
    ) as download_tmp:
        upload_dir = Path(upload_tmp)
        npm_dir = Path(npm_tmp)
        download_dir = Path(download_tmp)

        filecoin_pin_command, dependency_dir = setup_filecoin_pin(npm_dir)

        random_file = upload_dir / RAND_FILE_NAME
        info(f"Creating random file ({RAND_FILE_SIZE} bytes)")
        write_random_file(random_file, RAND_FILE_SIZE, RAND_FILE_SEED)
        assert_eq(
            random_file.stat().st_size,
            RAND_FILE_SIZE,
            f"{RAND_FILE_NAME} created with exact size {RAND_FILE_SIZE} bytes",
        )

        info("Running filecoin-pin multi-copy upload script against devnet")

        add_result = run_filecoin_pin_add(
            filecoin_pin_command,
            upload_dir,
            [],
            "default multi-copy",
        )
        if add_result is None:
            fail(f"filecoin-pin add --network devnet {upload_dir} did not run")
            return

        add_stdout = output_text(add_result.stdout).strip()
        add_stderr = output_text(add_result.stderr).strip()
        add_details_clean = strip_ansi(f"{add_stdout}\n{add_stderr}".strip())

        if add_result.returncode != 0:
            fail(f"""
                filecoin-pin add --network devnet {upload_dir} (exit={add_result.returncode})
                Retried for {ADD_DEADLINE_SECS}s.
                {add_details_clean}
                """.strip())
            return

        add_stdout_clean = strip_ansi(add_stdout)

        root_cid = None
        piece_cid = None
        piece_retrieval_urls = []
        parsed_root_cids = set()

        for line in add_stdout_clean.splitlines():
            stripped = line.strip()

            match = re.search(r"\broot CID:\s*(\S+)", stripped, re.IGNORECASE)
            if match:
                parsed_root_cids.add(match.group(1))
                if root_cid is None:
                    root_cid = match.group(1)

            if piece_cid is None and stripped.startswith("Piece CID:"):
                piece_cid = stripped.split(":", 1)[1].strip()

            if stripped.startswith("Retrieval URL:"):
                piece_retrieval_urls.append(stripped.split(":", 1)[1].strip())

        if not root_cid:
            fail(f"Could not parse Root CID from output: {add_details_clean}")
            return

        if len(parsed_root_cids) != 1:
            fail(
                f"filecoin-pin output returned different Root CIDs: {sorted(parsed_root_cids)}"
            )
            return

        if not piece_cid:
            fail(f"Could not parse Piece CID from output: {add_details_clean}")
            return

        if len(piece_retrieval_urls) < 2:
            fail(
                "Could not parse at least two Piece Retrieval URLs from output: "
                f"{add_details_clean}"
            )
            return

        root_retrieval_urls = []
        for url in piece_retrieval_urls:
            if "/piece/" not in url or piece_cid not in url:
                fail(
                    "Retrieval URL does not match the expected "
                    "/piece/<piece_cid> shape; cannot rewrite to /ipfs/<root_cid>: "
                    f"url={url} piece_cid={piece_cid}"
                )
                return
            root_retrieval_urls.append(
                url.replace("/piece/", "/ipfs/").replace(piece_cid, root_cid)
            )

        for i, url in enumerate(root_retrieval_urls, start=1):
            file = download_dir / f"{root_cid}_{i}.bin"
            error = download_and_verify(url, file, root_cid, dependency_dir)
            if error:
                fail(error)
                return
            info(f"Verified retrieval URL {i}: {url}")


if __name__ == "__main__":
    run()
