#!/usr/bin/env python3
"""Multi-copy upload test: upload a random file via filecoin-pin against the devnet."""

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


_CONTENT_LENGTH_LINE_RE = re.compile(
    r"^[ \t]*\.\.\.\(size == null \? \{\} : \{ 'Content-Length': size\.toString\(\) \}\),\r?\n",
    re.MULTILINE,
)


def patch_synapse_core_streaming_upload(npm_dir: Path) -> bool:
    """Strip Content-Length from @filoz/synapse-core's streaming upload headers.

    synapse-core (as of 0.4.1, which filecoin-pin 0.20.1 resolves to) sets a
    Content-Length header on a body that flows through a streaming Transform.
    Node/undici rejects that as 'invalid content-length header', which
    filecoin-pin surfaces as
    'StorageContext store failed: Failed to store piece on service provider -
    Network request failed'.

    TODO: drop this patch once filecoin-pin pins a synapse-core release that
    omits Content-Length on streaming bodies (track upstream in FilOzone/synapse-sdk).

    Returns True when the target file is in the desired state (whether or not
    a change was applied). Returns False only when the file is missing.
    """
    target = (
        npm_dir
        / "node_modules"
        / "@filoz"
        / "synapse-core"
        / "dist"
        / "src"
        / "sp"
        / "upload-streaming.js"
    )
    if not target.exists():
        return False

    text = target.read_text()
    updated, count = _CONTENT_LENGTH_LINE_RE.subn("", text)
    if count > 0:
        target.write_text(updated)
        info(
            "Patched @filoz/synapse-core streaming upload "
            f"(stripped {count} Content-Length line(s))"
        )
    else:
        info(
            "@filoz/synapse-core streaming upload already free of "
            "Content-Length header; no patch needed"
        )
    return True


def run_filecoin_pin_add(
    filecoin_pin_bin: Path,
    upload_dir: Path,
    extra_args: list[str],
    label: str,
):
    cmd = [
        str(filecoin_pin_bin),
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

        run_cmd(["npm", "init", "-y"], label="npm init", cwd=npm_dir)

        run_cmd(
            [
                "npm",
                "pkg",
                "set",
                "type=module",
                "dependencies.filecoin-pin=0.20.1",
                "dependencies.multiformats=13.4.2",
            ],
            label="pin filecoin-pin dependencies",
            cwd=npm_dir,
        )

        run_cmd(["npm", "install"], label="npm install", cwd=npm_dir)

        if not patch_synapse_core_streaming_upload(npm_dir):
            fail(
                "Could not locate @filoz/synapse-core streaming upload "
                "to patch (file missing under node_modules)"
            )
            return

        random_file = upload_dir / RAND_FILE_NAME
        info(f"Creating random file ({RAND_FILE_SIZE} bytes)")
        write_random_file(random_file, RAND_FILE_SIZE, RAND_FILE_SEED)
        assert_eq(
            random_file.stat().st_size,
            RAND_FILE_SIZE,
            f"{RAND_FILE_NAME} created with exact size {RAND_FILE_SIZE} bytes",
        )

        info("Running filecoin-pin multi-copy upload script against devnet")

        filecoin_pin_bin = npm_dir / "node_modules" / ".bin" / "filecoin-pin"

        add_result = run_filecoin_pin_add(
            filecoin_pin_bin,
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
            error = download_and_verify(url, file, root_cid, npm_dir)
            if error:
                fail(error)
                return
            info(f"Verified retrieval URL {i}: {url}")


if __name__ == "__main__":
    run()
