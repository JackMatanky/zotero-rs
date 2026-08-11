# zotero-mcp

MCP server that exposes Zotero library operations as tools for AI agents. Wraps zotero-api and zotero-semantic into an MCP interface using the rmcp SDK.

## Language

**MCP tool**:
A named function exposed by the MCP server that an AI agent can invoke. Each tool maps to a specific Zotero operation (search, create item, export note, etc.).
_Avoid_: function, command, endpoint

**MCP server**:
The process running zotero-mcp that listens for tool calls from AI agents. Communicates via the Model Context Protocol over stdio or transport-io.
_Avoid_: daemon, service

**Bridge**:
The companion JavaScript script (zotero-companion-bridge.js) that runs inside Zotero and exposes in-process APIs (Better Notes, file roots) to the Rust MCP server via HTTP.
_Avoid_: plugin, extension

**FileRoot**:
A kind/path pair representing a Zotero-managed filesystem root for PDFs. Three kinds: zotero-storage, zotero-linked-base, attanger-dest.
_Aavoid_: storage root, pdf directory

**Semantic search tool**:
MCP tool that calls zotero-semantic's search_library to perform natural-language queries against indexed item content.
_Aavoid_: vector search, embedding search
