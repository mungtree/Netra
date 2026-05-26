//! System-prompt fragment shared between the per-job resolver and the batch
//! reducer so the agent gets identical, correct ChromaDB usage instructions
//! in both contexts.

use std::path::Path;

/// Builds the system-prompt-append text that tells the pi agent how to query
/// ChromaDB. The pi process doesn't surface MCP tools to the model, so we
/// expose chroma through a small CLI (`chatur-chroma`) that the agent invokes
/// via its existing `bash` tool. `shim_path` is the absolute path of the
/// executable so the agent can invoke it verbatim without searching PATH.
#[must_use]
pub fn chromadb_system_prompt(collection_name: &str, shim_path: &Path) -> String {
    // On Windows the shim is a `.cmd` file. Rust's stdlib (since 1.77,
    // CVE-2024-24576) refuses to spawn `.bat`/`.cmd` with args it can't
    // safely escape — JSON in `--where`, quotes, `%`, etc. all trip the
    // `"batch file arguments are invalid"` error. Routing through
    // `cmd.exe /c` (a real `.exe`) bypasses the stdlib check and lets
    // cmd.exe parse the inner string itself.
    let shim = if cfg!(windows) {
        format!("cmd.exe /c \"{}\"", shim_path.display())
    } else {
        shim_path.display().to_string()
    };
    let platform_note = if cfg!(windows) {
        "\nPlatform: Windows. Your `bash` tool must shell out via `cmd.exe` \
        (the CLI path above already uses `cmd.exe /c`). When passing Windows paths as \
        arguments, escape every backslash as `\\\\` (e.g. `C:\\\\Users\\\\foo\\\\bar`) \
        so the shell doesn't eat them. Ensure you use cmd.exe fully, not just `cmd`.\n"
    } else {
        ""
    };
    format!(
        "ChromaDB has indexed this project's source code. Query it through \
the `chatur-chroma` CLI using your existing `bash` tool. There are NO \
chroma_* MCP tools — do not try to call them.\n\
\n\
Collection: {collection_name}\n\
CLI:        {shim}\n\
{platform_note}\
\n\
The CLI defaults `--collection` to this project's collection via the\n\
`CHATUR_CHROMA_COLLECTION` env var, so you can usually omit it.\n\
\n\
Subcommands (run via bash):\n\
\n\
  {shim} query --query \"<text>\" [--n 10] [--where '<json>']\n\
      PRIMARY semantic search. Output is one hit per line:\n\
          0.317  src/db/migrate.rs:42-78  applies pending migrations …\n\
      Columns: distance (lower = closer), path:line_start-line_end, snippet.\n\
      Add `--json` for {{\"hits\":[{{...}}]}} when you need structured output.\n\
\n\
  {shim} peek --n 5\n\
      Sample documents without a query — useful for a sanity check.\n\
\n\
  {shim} list\n\
      List every collection on the server.\n\
\n\
  {shim} info\n\
      Show count + metadata for the collection.\n\
\n\
  {shim} get [--ids id1,id2] [--where '<json>']\n\
      Fetch by id or metadata filter. `--where '{{\"path\":\"src/foo.rs\"}}'`\n\
      pulls every chunk of a specific file.\n\
\n\
Recommended workflow before answering questions about the codebase:\n\
  1. Run 2-4 `query` calls with different short phrasings of the intent\n\
     (e.g. \"sqlite migration runner\", \"applies pending migrations\"). Use\n\
     larger `--n` (10-20) for exploratory questions.\n\
  2. Read each hit's path with your normal `read` tool to confirm — chroma\n\
     chunks are ~800 chars and may be truncated.\n\
  3. Only fall back to grep / ls if `query` returns nothing relevant after\n\
     TWO distinct phrasings.\n\
\n\
If a `chatur-chroma` invocation fails, read the stderr verbatim and fix the\n\
arguments. Do not abandon chroma after one failure.",
    )
}
