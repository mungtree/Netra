//! System-prompt fragment shared between the per-job resolver and the batch
//! reducer so the agent gets identical, correct ChromaDB usage instructions
//! in both contexts.

/// Builds the system-prompt-append text that tells the pi agent how to use
/// the ChromaDB MCP server. The collection name is interpolated so the agent
/// never has to guess.
#[must_use]
pub fn chromadb_system_prompt(collection_name: &str) -> String {
    format!(
        "ChromaDB is available via the `chroma` MCP server. The current project's \
source code has been indexed into ONE collection named exactly:\n\
\n    {collection_name}\n\
\n\
Use these tools — the names are EXACT, do not invent variants:\n\
\n\
  chroma_list_collections()\n\
      Lists every collection on the server. Use to sanity-check.\n\
\n\
  chroma_query_documents(collection_name, query_texts, n_results=5,\n\
                         where=None,\n\
                         include=[\"documents\",\"metadatas\",\"distances\"])\n\
      PRIMARY semantic-search tool. `query_texts` is a LIST of short natural-\n\
      language queries (e.g. [\"sqlite migration runner\", \"applies pending\n\
      migrations\"]). Returns parallel arrays of documents, metadatas, and\n\
      distances. Each metadata entry contains `path`, `line_start`,\n\
      `line_end`, plus a `sha` of the source file.\n\
\n\
  chroma_peek_collection(collection_name, limit=5)\n\
      Sample documents without a query (useful for a sanity check).\n\
\n\
  chroma_get_documents(collection_name, ids=[...] OR where={{...}})\n\
      Fetch by id or metadata filter. Use `where={{\"path\": \"src/foo.rs\"}}`\n\
      to pull every chunk of a specific file.\n\
\n\
  chroma_get_collection_info(collection_name)\n\
      Returns collection metadata (count, embedding function, etc.).\n\
\n\
Recommended workflow before answering questions about the codebase:\n\
  1. Call chroma_query_documents with 2-4 short query_texts capturing the\n\
     intent. Larger n_results (10-20) for exploratory questions.\n\
  2. Read each hit's metadata.path. Open the actual file with your normal\n\
     `read` tool to confirm — chroma chunks are ~800 chars and may be\n\
     truncated.\n\
  3. Only fall back to directory listing / grep if chroma returns nothing\n\
     relevant after TWO distinct query phrasings.\n\
\n\
Do NOT invent other tool names. There is no `chroma_query`, no\n\
`chroma_search`, no `vector_search`. If a tool call fails, read the error\n\
verbatim and fix the arguments — do not abandon chroma after one failure."
    )
}
