#!/usr/bin/env python3
"""Bash-callable CLI the pi agent uses to query/inspect a chroma collection.

The agent doesn't have direct MCP access, so we expose chroma through a small
subcommand surface and let pi shell out to it via its `bash` tool.

Env (set by the chatur-chroma rust shim or by the resolver per pi spawn):

    CHATUR_CHROMA_HOST        defaults to 127.0.0.1
    CHATUR_CHROMA_PORT        defaults to 8765
    CHATUR_CHROMA_MODEL       defaults to "default" (must match the indexer
                              that built the collection or query fails on dim
                              mismatch)
    CHATUR_CHROMA_COLLECTION  optional default for --collection

Subcommands:

    chatur-chroma query   --query <text> [--collection <name>] [--n 10] [--where <json>]
    chatur-chroma peek    [--collection <name>] [--n 5]
    chatur-chroma list
    chatur-chroma info    [--collection <name>]
    chatur-chroma get     [--collection <name>] [--ids id1,id2] [--where <json>]

Add `--json` to any subcommand for machine-readable output.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from typing import Any

try:
    import chromadb
    from chromadb.utils import embedding_functions
except Exception as e:  # pragma: no cover
    print(f"chromadb import failed: {e}", file=sys.stderr)
    sys.exit(2)


def build_embedding_function(model_id: str):
    if model_id == "default":
        return embedding_functions.DefaultEmbeddingFunction()
    # trust_remote_code=True: Hub models with custom modeling code (e.g.
    # jinaai/jina-embeddings-v2-*) ship their own modeling_bert.py.
    return embedding_functions.SentenceTransformerEmbeddingFunction(
        model_name=model_id,
        trust_remote_code=True,
    )


def resolve_collection(arg: str | None) -> str:
    if arg:
        return arg
    env = os.environ.get("CHATUR_CHROMA_COLLECTION")
    if env:
        return env
    print(
        "error: --collection not given and CHATUR_CHROMA_COLLECTION not set",
        file=sys.stderr,
    )
    sys.exit(64)


def open_client_and_collection(collection: str):
    host = os.environ.get("CHATUR_CHROMA_HOST", "127.0.0.1")
    port = int(os.environ.get("CHATUR_CHROMA_PORT", "8765"))
    model = os.environ.get("CHATUR_CHROMA_MODEL", "default")
    ef = build_embedding_function(model)
    client = chromadb.HttpClient(host=host, port=port)
    coll = client.get_or_create_collection(name=collection, embedding_function=ef)
    return client, coll


def snippet(text: str, max_len: int = 140) -> str:
    one_line = " ".join(text.split())
    if len(one_line) <= max_len:
        return one_line
    return one_line[: max_len - 1] + "…"


def parse_where(raw: str | None) -> Any:
    if not raw:
        return None
    try:
        return json.loads(raw)
    except json.JSONDecodeError as e:
        print(f"error: --where must be JSON: {e}", file=sys.stderr)
        sys.exit(64)


def cmd_query(args: argparse.Namespace) -> int:
    name = resolve_collection(args.collection)
    _, coll = open_client_and_collection(name)
    res = coll.query(
        query_texts=[args.query],
        n_results=args.n,
        where=parse_where(args.where),
        include=["documents", "metadatas", "distances"],
    )
    ids = (res.get("ids") or [[]])[0]
    docs = (res.get("documents") or [[]])[0] or []
    metas = (res.get("metadatas") or [[]])[0] or []
    dists = (res.get("distances") or [[]])[0] or []

    hits = []
    for i, _id in enumerate(ids):
        meta = metas[i] if i < len(metas) else {}
        hits.append({
            "id": _id,
            "distance": float(dists[i]) if i < len(dists) else 0.0,
            "path": meta.get("path", ""),
            "line_start": meta.get("line_start"),
            "line_end": meta.get("line_end"),
            "document": docs[i] if i < len(docs) else "",
            "metadata": meta,
        })

    if args.json:
        print(json.dumps({"hits": hits}))
    else:
        if not hits:
            print("(no hits)")
            return 0
        for h in hits:
            path = h["path"] or h["id"]
            ls, le = h["line_start"], h["line_end"]
            loc = f"{path}:{ls}-{le}" if ls is not None else path
            print(f"{h['distance']:.3f}  {loc}  {snippet(h['document'])}")
    return 0


def cmd_peek(args: argparse.Namespace) -> int:
    name = resolve_collection(args.collection)
    _, coll = open_client_and_collection(name)
    res = coll.peek(limit=args.n)
    ids = res.get("ids") or []
    docs = res.get("documents") or []
    metas = res.get("metadatas") or []
    items = [
        {
            "id": _id,
            "path": (metas[i] if i < len(metas) else {}).get("path", ""),
            "document": docs[i] if i < len(docs) else "",
            "metadata": metas[i] if i < len(metas) else {},
        }
        for i, _id in enumerate(ids)
    ]
    if args.json:
        print(json.dumps({"items": items}))
    else:
        for it in items:
            print(f"{it['id']}  {it['path']}  {snippet(it['document'])}")
    return 0


def cmd_list(args: argparse.Namespace) -> int:
    host = os.environ.get("CHATUR_CHROMA_HOST", "127.0.0.1")
    port = int(os.environ.get("CHATUR_CHROMA_PORT", "8765"))
    client = chromadb.HttpClient(host=host, port=port)
    names = [c.name for c in client.list_collections()]
    if args.json:
        print(json.dumps({"collections": names}))
    else:
        for n in names:
            print(n)
    return 0


def cmd_info(args: argparse.Namespace) -> int:
    name = resolve_collection(args.collection)
    _, coll = open_client_and_collection(name)
    info = {
        "name": coll.name,
        "count": coll.count(),
        "metadata": coll.metadata or {},
    }
    if args.json:
        print(json.dumps(info))
    else:
        print(f"name:     {info['name']}")
        print(f"count:    {info['count']}")
        if info["metadata"]:
            print(f"metadata: {json.dumps(info['metadata'])}")
    return 0


def cmd_get(args: argparse.Namespace) -> int:
    name = resolve_collection(args.collection)
    _, coll = open_client_and_collection(name)
    ids = [s for s in (args.ids or "").split(",") if s] or None
    where = parse_where(args.where)
    res = coll.get(
        ids=ids,
        where=where,
        include=["documents", "metadatas"],
    )
    ids_out = res.get("ids") or []
    docs = res.get("documents") or []
    metas = res.get("metadatas") or []
    items = [
        {
            "id": _id,
            "path": (metas[i] if i < len(metas) else {}).get("path", ""),
            "document": docs[i] if i < len(docs) else "",
            "metadata": metas[i] if i < len(metas) else {},
        }
        for i, _id in enumerate(ids_out)
    ]
    if args.json:
        print(json.dumps({"items": items}))
    else:
        for it in items:
            print(f"{it['id']}  {it['path']}  {snippet(it['document'])}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="chatur-chroma")
    p.add_argument("--json", action="store_true", help="machine-readable output")
    sub = p.add_subparsers(dest="cmd", required=True)

    q = sub.add_parser("query", help="semantic search the collection")
    q.add_argument("--query", required=True)
    q.add_argument("--collection", default=None)
    q.add_argument("--n", type=int, default=10)
    q.add_argument("--where", default=None, help="JSON metadata filter")
    q.set_defaults(func=cmd_query)

    pk = sub.add_parser("peek", help="sample documents without a query")
    pk.add_argument("--collection", default=None)
    pk.add_argument("--n", type=int, default=5)
    pk.set_defaults(func=cmd_peek)

    ls = sub.add_parser("list", help="list collections on the server")
    ls.set_defaults(func=cmd_list)

    inf = sub.add_parser("info", help="collection metadata + count")
    inf.add_argument("--collection", default=None)
    inf.set_defaults(func=cmd_info)

    g = sub.add_parser("get", help="fetch documents by id or where filter")
    g.add_argument("--collection", default=None)
    g.add_argument("--ids", default=None, help="comma-separated ids")
    g.add_argument("--where", default=None, help="JSON metadata filter")
    g.set_defaults(func=cmd_get)
    return p


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    try:
        return args.func(args)
    except Exception as e:
        print(f"chatur-chroma error: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
