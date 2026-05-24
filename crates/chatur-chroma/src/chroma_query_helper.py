#!/usr/bin/env python3
"""Query a chroma collection. Chroma 1.x server requires client-side
embeddings, so we instantiate the same embedding function that was used at
index time (selected via the 4th CLI arg).

- "default" => DefaultEmbeddingFunction (ONNX all-MiniLM-L6-v2, 384d).
- Otherwise treated as a HuggingFace model id loaded via
  SentenceTransformerEmbeddingFunction.

Reads a JSON request from stdin:
    {"query_texts": ["..."], "n_results": 10, "where": null}

Writes a JSON response to stdout:
    {"hits": [{"id":"...","distance":0.31,"document":"...","metadata":{...}}]}

Usage:
    python helper.py <host> <port> <collection_name> <model_id>
"""
import sys
import json

try:
    import chromadb
    from chromadb.utils import embedding_functions
except Exception as e:
    print(f"chromadb import failed: {e}", file=sys.stderr)
    sys.exit(2)


def build_embedding_function(model_id: str):
    if model_id == "default":
        return embedding_functions.DefaultEmbeddingFunction()
    try:
        return embedding_functions.SentenceTransformerEmbeddingFunction(
            model_name=model_id
        )
    except Exception as e:
        print(
            f"failed to load embedding model {model_id!r}: {e}",
            file=sys.stderr,
        )
        raise


def main() -> int:
    if len(sys.argv) != 5:
        print(
            "usage: helper.py <host> <port> <collection> <model_id>",
            file=sys.stderr,
        )
        return 64

    host, port_s, collection, model_id = sys.argv[1:5]
    try:
        port = int(port_s)
    except ValueError:
        print(f"invalid port: {port_s}", file=sys.stderr)
        return 64

    payload = json.load(sys.stdin)
    query_texts = payload["query_texts"]
    n_results = int(payload.get("n_results", 10))
    where = payload.get("where")

    ef = build_embedding_function(model_id)
    client = chromadb.HttpClient(host=host, port=port)
    coll = client.get_or_create_collection(name=collection, embedding_function=ef)
    res = coll.query(
        query_texts=query_texts,
        n_results=n_results,
        where=where,
        include=["documents", "metadatas", "distances"],
    )

    # chroma returns parallel lists-of-lists keyed by query index. Flatten
    # the first query's results into hit objects (callers only send one
    # query_text at a time from the UI).
    ids = (res.get("ids") or [[]])[0]
    docs = (res.get("documents") or [[]])[0] or []
    metas = (res.get("metadatas") or [[]])[0] or []
    dists = (res.get("distances") or [[]])[0] or []

    hits = []
    for i, _id in enumerate(ids):
        hits.append({
            "id": _id,
            "distance": float(dists[i]) if i < len(dists) else 0.0,
            "document": docs[i] if i < len(docs) else "",
            "metadata": metas[i] if i < len(metas) else {},
        })
    print(json.dumps({"hits": hits}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
