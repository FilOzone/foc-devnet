#!/usr/bin/env python3
"""Multi-copy upload test: upload a random file via filecoin-pin against the devnet."""

import os
import subprocess
import sys
import tempfile
from pathlib import Path
from urllib.error import HTTPError, URLError
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

RAND_FILE_NAME = "random_file"
RAND_FILE_SIZE = 20 * 1024 * 1024
RAND_FILE_SEED = 42


def run():
    assert_ok("command -v node", "node is installed")
    assert_ok("command -v pnpm", "pnpm is installed")

    scripts_dir = Path("scripts").resolve()

    with tempfile.TemporaryDirectory(prefix="filecoin-pin-") as tmp:
        if not run_cmd(
            ["npm", "install", "-g", "filecoin-pin"],
            label="npm install -g filecoin-pin",
        ):
            return
        if not run_cmd(
            ["npm", "install", "-g", "multiformats"],
            label="npm install -g multiformats",
        ):
            return

        random_file = Path(tmp) / RAND_FILE_NAME
        info(f"Creating random file ({RAND_FILE_SIZE} bytes)")
        write_random_file(random_file, RAND_FILE_SIZE, RAND_FILE_SEED)
        assert_eq(
            random_file.stat().st_size,
            RAND_FILE_SIZE,
            f"{RAND_FILE_NAME} created with exact size {RAND_FILE_SIZE} bytes",
        )

        info("Running filecoin-pin multi-copy upload script against devnet")

        add_result = subprocess.run(
            ["filecoin-pin", "add", "--network", "devnet", tmp],
            text=True,
            capture_output=True,
        )
        add_stdout = (add_result.stdout or "").strip()
        add_stderr = (add_result.stderr or "").strip()
        if add_result.returncode != 0:
            fail(
                f"""
                filecoin-pin add --network devnet {tmp} (exit={add_result.returncode})
                {add_stdout}
                {add_stderr}
                """.strip()
            )
            return

        root_cid = None
        piece_cid = None
        piece_retrieval_urls = []

        for line in add_stdout.splitlines():
            stripped = line.strip()

            if root_cid is None and stripped.startswith("Root CID:"):
                root_cid = stripped.split(":", 1)[1].strip()

            if piece_cid is None and stripped.startswith("Piece CID:"):
                piece_cid = stripped.split(":", 1)[1].strip()

            if stripped.startswith("Retrieval URL:"):
                piece_retrieval_urls.append(stripped.split(":", 1)[1].strip())

        if not root_cid:
            fail(f"Could not parse Root CID from output: {add_details}")
            return

        if not piece_cid:
            fail(f"Could not parse Piece CID from output: {add_details}")
            return

        if not piece_retrieval_urls:
            fail(f"Could not parse Piece Retrieval URLs from output: {add_details}")
            return

        root_retrieval_urls = [
            url.replace("/piece/", "/ipfs/").replace(piece_cid, root_cid)
            for url in piece_retrieval_urls
        ]

        ipfs_dir = Path("ipfs")
        ipfs_dir.mkdir(parents=True, exist_ok=True)

        for i, url in enumerate(root_retrieval_urls, start=1):
            file = ipfs_dir / f"{root_cid}_{i}.bin"
            download_url = f"{url}?format=raw"

            try:
                with urlopen(download_url, timeout=60) as resp, open(file, "wb") as f:
                    while True:
                        chunk = resp.read(1024 * 1024)
                        if not chunk:
                            break
                        f.write(chunk)
            except (URLError, OSError) as e:
                fail(f"Failed to download {download_url}: {e}")
                return

            if not run_cmd(
                ["node", str(scripts_dir / "verify_cid.mjs"), root_cid, str(file)],
                label=f"node {scripts_dir / 'verify_cid.mjs'} {root_cid} {file} ({download_url})",
            ):
                return


if __name__ == "__main__":
    run()
