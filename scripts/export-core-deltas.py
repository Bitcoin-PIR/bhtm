#!/usr/bin/env python3
"""Export Bitcoin Core blocks as compact stripped-block MuHash deltas.

This is the source-side extractor for the slim verifier. It talks to a trusted
local Bitcoin Core RPC, parses raw blocks, maintains the UTXO map needed to
derive spent-coin metadata, and writes the compact binary stream consumed by
`blockhash-to-muhash bin`.

The verifier still owns the light-client-style checks. This exporter owns the
Bitcoin Core UTXO semantics: genesis exclusion, unspendable-output exclusion,
spent coin metadata, and historical duplicate-txid overwrites.
"""

from __future__ import annotations

import argparse
import base64
import json
import struct
import sys
import time
import urllib.request
from dataclasses import dataclass
from decimal import Decimal
from pathlib import Path
from typing import BinaryIO
from hashlib import sha256


BINARY_MAGIC = b"BHTMBIN1"
ANCHOR_BEFORE_GENESIS = 0
ANCHOR_TRUSTED_BLOCK = 1
BIP30_DUPLICATE_OVERWRITE_LAST_HEIGHT = 91880
MAX_RPC_BATCH = 250
PREVOUT_RPC_BATCH = 25
SATOSHIS_PER_BTC = Decimal("100000000")


@dataclass(frozen=True)
class TxInput:
    prev_txid: bytes
    prev_vout: int


@dataclass(frozen=True)
class TxOutput:
    amount: int
    script_pubkey: bytes


@dataclass(frozen=True)
class Tx:
    stripped: bytes
    txid: bytes
    inputs: list[TxInput]
    outputs: list[TxOutput]
    is_coinbase: bool


@dataclass(frozen=True)
class Coin:
    txid: bytes
    vout: int
    amount: int
    height: int
    is_coinbase: bool
    script_pubkey: bytes


class CountingWriter:
    def __init__(self, inner: BinaryIO):
        self.inner = inner
        self.bytes_written = 0

    def write(self, data: bytes) -> None:
        self.inner.write(data)
        self.bytes_written += len(data)

    def flush(self) -> None:
        self.inner.flush()


def sha256d(data: bytes) -> bytes:
    return sha256(sha256(data).digest()).digest()


def read_compact_size(data: bytes, offset: int) -> tuple[int, int, bytes]:
    first = data[offset]
    if first < 0xFD:
        return first, offset + 1, data[offset : offset + 1]
    if first == 0xFD:
        end = offset + 3
        return struct.unpack_from("<H", data, offset + 1)[0], end, data[offset:end]
    if first == 0xFE:
        end = offset + 5
        return struct.unpack_from("<I", data, offset + 1)[0], end, data[offset:end]
    end = offset + 9
    return struct.unpack_from("<Q", data, offset + 1)[0], end, data[offset:end]


def parse_tx(data: bytes, offset: int) -> tuple[Tx, int]:
    start = offset
    version = data[offset : offset + 4]
    offset += 4

    has_witness = False
    if offset + 2 <= len(data) and data[offset] == 0 and data[offset + 1] != 0:
        has_witness = True
        offset += 2

    vin_count, offset, vin_count_ser = read_compact_size(data, offset)
    stripped_parts = [version, vin_count_ser]
    inputs: list[TxInput] = []

    for _ in range(vin_count):
        input_start = offset
        prev_txid = data[offset : offset + 32]
        offset += 32
        prev_vout = struct.unpack_from("<I", data, offset)[0]
        offset += 4
        script_len, offset, _ = read_compact_size(data, offset)
        offset += script_len
        offset += 4  # sequence
        stripped_parts.append(data[input_start:offset])
        inputs.append(TxInput(prev_txid, prev_vout))

    vout_count, offset, vout_count_ser = read_compact_size(data, offset)
    stripped_parts.append(vout_count_ser)
    outputs: list[TxOutput] = []

    for _ in range(vout_count):
        output_start = offset
        amount = struct.unpack_from("<q", data, offset)[0]
        offset += 8
        script_len, offset, _ = read_compact_size(data, offset)
        script_pubkey = data[offset : offset + script_len]
        offset += script_len
        stripped_parts.append(data[output_start:offset])
        outputs.append(TxOutput(amount, script_pubkey))

    if has_witness:
        for _ in range(vin_count):
            item_count, offset, _ = read_compact_size(data, offset)
            for _ in range(item_count):
                item_len, offset, _ = read_compact_size(data, offset)
                offset += item_len

    locktime = data[offset : offset + 4]
    offset += 4
    stripped_parts.append(locktime)

    stripped = b"".join(stripped_parts)
    is_coinbase = (
        len(inputs) == 1
        and inputs[0].prev_txid == b"\x00" * 32
        and inputs[0].prev_vout == 0xFFFFFFFF
    )
    return Tx(stripped, sha256d(stripped), inputs, outputs, is_coinbase), offset


def parse_block(raw: bytes) -> tuple[bytes, bytes, list[Tx]]:
    header = raw[:80]
    offset = 80
    tx_count, offset, _ = read_compact_size(raw, offset)
    txs = []
    for _ in range(tx_count):
        tx, offset = parse_tx(raw, offset)
        txs.append(tx)
    if offset != len(raw):
        raise ValueError(f"block parser stopped at {offset}, raw length is {len(raw)}")
    return header, sha256d(header), txs


def write_u8(out: CountingWriter, value: int) -> None:
    out.write(struct.pack("<B", value))


def write_u32(out: CountingWriter, value: int) -> None:
    out.write(struct.pack("<I", value))


def write_u64(out: CountingWriter, value: int) -> None:
    out.write(struct.pack("<Q", value))


def write_bytes(out: CountingWriter, data: bytes) -> None:
    write_u32(out, len(data))
    out.write(data)


def write_coin(out: CountingWriter, coin: Coin) -> None:
    out.write(coin.txid)
    write_u32(out, coin.vout)
    write_u64(out, coin.amount)
    write_u32(out, coin.height)
    write_u8(out, 1 if coin.is_coinbase else 0)
    write_bytes(out, coin.script_pubkey)


def write_header(
    out: CountingWriter,
    anchor_kind: int,
    start_height: int,
    start_block_hash: bytes,
    start_muhash: bytes,
) -> None:
    out.write(BINARY_MAGIC)
    write_u8(out, anchor_kind)
    write_u32(out, start_height)
    out.write(start_block_hash)
    out.write(start_muhash)


def write_block(
    out: CountingWriter,
    height: int,
    header: bytes,
    block_hash: bytes,
    txs: list[Tx],
    additions: list[Coin],
    removals: list[Coin],
) -> None:
    write_u32(out, height)
    out.write(header)
    out.write(block_hash)
    write_u32(out, len(txs))
    for tx in txs:
        write_bytes(out, tx.stripped)
    write_u32(out, len(additions))
    for coin in additions:
        write_coin(out, coin)
    write_u32(out, len(removals))
    for coin in removals:
        write_coin(out, coin)


def is_unspendable(script_pubkey: bytes) -> bool:
    return bool(script_pubkey) and script_pubkey[0] == 0x6A


class Rpc:
    def __init__(self, url: str, auth: str):
        self.url = url
        self.auth_header = "Basic " + base64.b64encode(auth.encode()).decode()

    def batch(self, calls: list[tuple[str, list]]) -> list:
        payload = [
            {"jsonrpc": "1.0", "id": i, "method": method, "params": params}
            for i, (method, params) in enumerate(calls)
        ]
        req = urllib.request.Request(
            self.url,
            json.dumps(payload).encode(),
            {"Content-Type": "application/json", "Authorization": self.auth_header},
        )
        with urllib.request.urlopen(req, timeout=300) as resp:
            replies = json.loads(resp.read(), parse_float=Decimal)
        replies.sort(key=lambda item: item["id"])
        out = []
        for reply in replies:
            if reply.get("error"):
                raise RuntimeError(f"RPC error in call {reply['id']}: {reply['error']}")
            out.append(reply["result"])
        return out


def rpc_from_args(args: argparse.Namespace) -> Rpc:
    if args.rpc_auth:
        auth = args.rpc_auth
    else:
        cookie_path = Path(args.datadir) / ".cookie"
        auth = cookie_path.read_text().strip()
    return Rpc(args.rpc_url, auth)


def display_hash_to_internal(display_hash: str) -> bytes:
    raw = bytes.fromhex(display_hash)
    if len(raw) != 32:
        raise ValueError(f"expected 32-byte block/tx hash, got {len(raw)} bytes")
    return raw[::-1]


def parse_muhash_hex(value: str) -> bytes:
    raw = bytes.fromhex(value)
    if len(raw) != 384:
        raise ValueError(f"expected 384-byte MuHash value, got {len(raw)} bytes")
    return raw


def btc_to_sats(value: Decimal | int | float | str) -> int:
    sats = (Decimal(str(value)) * SATOSHIS_PER_BTC).to_integral_exact()
    return int(sats)


def build_additions(height: int, txs: list[Tx], stats: dict) -> list[Coin]:
    additions: list[Coin] = []
    if height == 0:
        return additions

    for tx in txs:
        for vout, txout in enumerate(tx.outputs):
            if is_unspendable(txout.script_pubkey):
                stats["unspendable_outputs_skipped"] += 1
                continue

            additions.append(
                Coin(
                    txid=tx.txid,
                    vout=vout,
                    amount=txout.amount,
                    height=height,
                    is_coinbase=tx.is_coinbase,
                    script_pubkey=txout.script_pubkey,
                )
            )
    return additions


def build_prevout_removals(height: int, txs: list[Tx], verbose_txs: list[dict]) -> list[Coin]:
    if len(txs) != len(verbose_txs):
        raise ValueError(
            f"raw/verbose tx count mismatch at height {height}: "
            f"{len(txs)} != {len(verbose_txs)}"
        )

    removals: list[Coin] = []
    for tx, verbose_tx in zip(txs, verbose_txs):
        display_txid = tx.txid[::-1].hex()
        if verbose_tx.get("txid") != display_txid:
            raise ValueError(
                f"raw/verbose txid mismatch at height {height}: "
                f"{display_txid} != {verbose_tx.get('txid')}"
            )
        if tx.is_coinbase:
            continue

        for txin in verbose_tx.get("vin", []):
            if "coinbase" in txin:
                continue
            prevout = txin.get("prevout")
            if prevout is None:
                raise ValueError(
                    "missing prevout data; getblock verbosity 3 requires "
                    f"unpruned undo data on the active chain at height {height}"
                )
            script = bytes.fromhex(prevout["scriptPubKey"]["hex"])
            removals.append(
                Coin(
                    txid=display_hash_to_internal(txin["txid"]),
                    vout=int(txin["vout"]),
                    amount=btc_to_sats(prevout["value"]),
                    height=int(prevout["height"]),
                    is_coinbase=bool(prevout["generated"]),
                    script_pubkey=script,
                )
            )
    return removals


def export(args: argparse.Namespace) -> dict:
    if args.end_height < args.start_height:
        raise ValueError("end-height must be greater than or equal to start-height")
    if args.end_height < 0:
        raise ValueError("end-height must be non-negative")

    source_mode = args.source_mode
    if source_mode == "auto":
        source_mode = "utxo-scan" if args.start_height == 0 else "prevout-rpc"

    if source_mode == "utxo-scan" and args.start_height != 0:
        raise ValueError("utxo-scan mode currently supports only start-height 0")
    if source_mode == "prevout-rpc" and args.start_height <= BIP30_DUPLICATE_OVERWRITE_LAST_HEIGHT:
        raise ValueError(
            "prevout-rpc mode intentionally refuses ranges that overlap the "
            "historical BIP30 duplicate-txid overwrite heights; use utxo-scan "
            "from genesis for the first chunk"
        )
    batch_size = args.batch_size
    if batch_size is None:
        batch_size = PREVOUT_RPC_BATCH if source_mode == "prevout-rpc" else MAX_RPC_BATCH

    if args.start_height == 0:
        anchor_kind = ANCHOR_BEFORE_GENESIS
        anchor_height = 0
        anchor_block_hash = b"\x00" * 32
        anchor_muhash = b"\x01" + b"\x00" * 383
    else:
        if args.anchor_height is None or args.anchor_block_hash is None or args.anchor_muhash is None:
            raise ValueError(
                "start-height > 0 requires --anchor-height, --anchor-block-hash, "
                "and --anchor-muhash"
            )
        if args.anchor_height + 1 != args.start_height:
            raise ValueError("anchor-height must be exactly start-height - 1")
        anchor_kind = ANCHOR_TRUSTED_BLOCK
        anchor_height = args.anchor_height
        anchor_block_hash = display_hash_to_internal(args.anchor_block_hash)
        anchor_muhash = parse_muhash_hex(args.anchor_muhash)

    rpc = rpc_from_args(args)
    output = sys.stdout.buffer if args.out == "-" else open(args.out, "wb")

    utxos: dict[tuple[bytes, int], Coin] = {}
    stats = {
        "source_mode": source_mode,
        "batch_size": batch_size,
        "anchor_kind": anchor_kind,
        "anchor_height": anchor_height,
        "anchor_block_hash_display": anchor_block_hash[::-1].hex(),
        "first_block_height": args.start_height,
        "end_height": args.end_height,
        "blocks": 0,
        "transactions": 0,
        "coins_added": 0,
        "coins_removed": 0,
        "unspendable_outputs_skipped": 0,
        "duplicate_overwrites": 0,
        "duplicate_overwrite_sats": 0,
        "max_utxos": 0,
        "input_bytes": 0,
        "output_bytes": 0,
        "elapsed_secs": 0.0,
    }

    started = time.time()
    try:
        out = CountingWriter(output)
        write_header(out, anchor_kind, anchor_height, anchor_block_hash, anchor_muhash)

        for first in range(args.start_height, args.end_height + 1, batch_size):
            last = min(args.end_height, first + batch_size - 1)
            heights = list(range(first, last + 1))
            hashes = rpc.batch([("getblockhash", [height]) for height in heights])
            raw_hexes = rpc.batch([("getblock", [block_hash, 0]) for block_hash in hashes])
            verbose_blocks = (
                rpc.batch([("getblock", [block_hash, 3]) for block_hash in hashes])
                if source_mode == "prevout-rpc"
                else [None] * len(hashes)
            )

            for height, display_hash, raw_hex, verbose_block in zip(
                heights, hashes, raw_hexes, verbose_blocks
            ):
                raw = bytes.fromhex(raw_hex)
                header, block_hash, txs = parse_block(raw)
                if block_hash[::-1].hex() != display_hash:
                    raise ValueError(
                        f"RPC/display block hash mismatch at height {height}: "
                        f"{block_hash[::-1].hex()} != {display_hash}"
                    )

                additions: list[Coin] = []
                removals: list[Coin] = []

                if source_mode == "utxo-scan":
                    for tx in txs:
                        if not tx.is_coinbase:
                            for txin in tx.inputs:
                                key = (txin.prev_txid, txin.prev_vout)
                                coin = utxos.pop(key, None)
                                if coin is None:
                                    raise KeyError(
                                        f"missing spent coin at height {height}: "
                                        f"{txin.prev_txid[::-1].hex()}:{txin.prev_vout}"
                                    )
                                removals.append(coin)

                        if height == 0:
                            continue

                        for vout, txout in enumerate(tx.outputs):
                            if is_unspendable(txout.script_pubkey):
                                stats["unspendable_outputs_skipped"] += 1
                                continue

                            key = (tx.txid, vout)
                            old_coin = utxos.pop(key, None)
                            if old_coin is not None:
                                removals.append(old_coin)
                                stats["duplicate_overwrites"] += 1
                                stats["duplicate_overwrite_sats"] += old_coin.amount

                            coin = Coin(
                                txid=tx.txid,
                                vout=vout,
                                amount=txout.amount,
                                height=height,
                                is_coinbase=tx.is_coinbase,
                                script_pubkey=txout.script_pubkey,
                            )
                            utxos[key] = coin
                            additions.append(coin)
                else:
                    if verbose_block is None or int(verbose_block["height"]) != height:
                        raise ValueError(f"verbose block height mismatch at height {height}")
                    removals = build_prevout_removals(height, txs, verbose_block["tx"])
                    additions = build_additions(height, txs, stats)

                write_block(out, height, header, block_hash, txs, additions, removals)

                stats["blocks"] += 1
                stats["transactions"] += len(txs)
                stats["coins_added"] += len(additions)
                stats["coins_removed"] += len(removals)
                stats["input_bytes"] += len(raw)
                if source_mode == "utxo-scan":
                    stats["max_utxos"] = max(stats["max_utxos"], len(utxos))

                if (
                    args.progress_interval
                    and (height == args.start_height or (height + 1) % args.progress_interval == 0)
                ):
                    elapsed = max(time.time() - started, 1e-9)
                    print(
                        f"height={height} blocks={stats['blocks']} txs={stats['transactions']} "
                        f"+{stats['coins_added']} -{stats['coins_removed']} "
                        f"utxos={len(utxos)} bytes={out.bytes_written} "
                        f"rate={stats['blocks'] / elapsed:.1f} blk/s",
                        file=sys.stderr,
                        flush=True,
                    )

        out.flush()
        stats["output_bytes"] = out.bytes_written
    finally:
        if output is not sys.stdout.buffer:
            output.close()

    stats["elapsed_secs"] = time.time() - started
    stats["final_utxos"] = len(utxos) if source_mode == "utxo-scan" else None
    stats["net_utxo_delta"] = stats["coins_added"] - stats["coins_removed"]
    return stats


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--datadir", default="/Volumes/Bitcoin/bitcoin")
    parser.add_argument("--rpc-url", default="http://127.0.0.1:8332/")
    parser.add_argument("--rpc-auth", help="RPC user:password; defaults to datadir .cookie")
    parser.add_argument("--start-height", type=int, default=0)
    parser.add_argument("--end-height", type=int, required=True)
    parser.add_argument(
        "--source-mode",
        choices=["auto", "utxo-scan", "prevout-rpc"],
        default="auto",
        help="spent-coin source; auto uses utxo-scan for genesis and Core prevout RPC otherwise",
    )
    parser.add_argument("--anchor-height", type=int, help="trusted anchor height for start-height > 0")
    parser.add_argument(
        "--anchor-block-hash",
        help="trusted anchor block hash in Bitcoin display/RPC byte order",
    )
    parser.add_argument("--anchor-muhash", help="trusted full 384-byte anchor MuHash hex")
    parser.add_argument(
        "--batch-size",
        type=int,
        help=(
            f"RPC batch size; defaults to {MAX_RPC_BATCH} in utxo-scan mode "
            f"and {PREVOUT_RPC_BATCH} in prevout-rpc mode"
        ),
    )
    parser.add_argument("--progress-interval", type=int, default=1000)
    parser.add_argument("--out", default="-", help="output path, or '-' for stdout")
    parser.add_argument("--stats-out", help="write exporter stats JSON to this path")
    args = parser.parse_args()

    try:
        stats = export(args)
    except BrokenPipeError:
        sys.exit(141)

    stats_json = json.dumps(stats, indent=2, sort_keys=True)
    if args.stats_out:
        Path(args.stats_out).write_text(stats_json + "\n")
    print(stats_json, file=sys.stderr)


if __name__ == "__main__":
    main()
