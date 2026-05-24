#!/usr/bin/env python3
"""Helper invoked by chatur-chroma to upsert chunks into a chroma collection.

Reads a JSON batch from stdin:
    {"ids": [...], "documents": [...], "metadatas": [...]}

The chromadb python client computes embeddings client-side and POSTs them to
the running server. The 4th CLI arg selects the embedding function:

- "default" => chromadb's built-in DefaultEmbeddingFunction (ONNX
  all-MiniLM-L6-v2, 384d, no extra download).
- Any other value is treated as a HuggingFace model id and loaded via
  SentenceTransformerEmbeddingFunction (downloaded to ~/.cache on first use).

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
    ids = payload["ids"]
    documents = payload["documents"]
    metadatas = payload.get("metadatas")

    ef = build_embedding_function(model_id)
    client = chromadb.HttpClient(host=host, port=port)
    coll = client.get_or_create_collection(name=collection, embedding_function=ef)
    coll.upsert(ids=ids, documents=documents, metadatas=metadatas)
    print(json.dumps({"upserted": len(ids)}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
