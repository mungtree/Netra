// Chatur — Run Review page
const { useState, useMemo } = React;
const R = window.CHATUR_REVIEW;

/* ---------- Shared chrome ---------- */
function Titlebar() {
  return (
    <div className="titlebar">
      <div className="traffic"><span/><span/><span/></div>
      <div className="brand">
        <span className="dot"/>
        <span className="name">chatur</span>
        <span className="v">v0.4.2</span>
      </div>
      <div className="titlebar-spacer"/>
      <div className="titlebar-status">
        <span className="pill"><span className="pulse"/>llama.cpp · local</span>
        <span>⌘K to search</span>
      </div>
    </div>
  );
}

function ActivityBar() {
  const items = [
    { id: "projects", icon: "folder", href: "Chatur.html" },
    { id: "library",  icon: "library" },
    { id: "history",  icon: "clock", active: true },
    { id: "activity", icon: "activity" },
  ];
  return (
    <div className="activitybar">
      {items.map(i => (
        <a
          key={i.id}
          href={i.href || "#"}
          className={`ab-btn${i.active ? " active" : ""}`}
          title={i.id}
        >
          <Icon name={i.icon}/>
        </a>
      ))}
      <div className="ab-spacer"/>
      <button className="ab-btn" title="Settings"><Icon name="settings"/></button>
    </div>
  );
}

function RecentSidebar({ activeId, onPick }) {
  return (
    <div className="sidebar">
      <div className="sb-header">
        <div className="sb-title">Recent Runs</div>
        <button className="sb-add" title="Filter"><Icon name="search" size={13}/></button>
      </div>
      <div className="sb-search">
        <Icon name="search" size={13}/>
        <input placeholder="Search runs…"/>
        <span className="kbd">⌘F</span>
      </div>
      <div className="proj-list">
        {R.recentRuns.map(r => (
          <div
            key={r.id}
            className={`proj-item${r.id === activeId ? " active" : ""}`}
            onClick={() => onPick(r.id)}
          >
            <span className={`proj-status ${r.highSev > 0 ? "err" : "done"}`}/>
            <div className="proj-info">
              <div className="proj-name">{r.task}</div>
              <div className="proj-path">{r.project} · {r.when}</div>
            </div>
            <div className="proj-count">
              {r.findings || ""}
            </div>
          </div>
        ))}
      </div>
      <div className="sb-footer">
        <div className="model-chip">
          <Icon name="cpu" size={12}/>
          <span><span className="auto">AUTO</span> · <span className="mname">qwen2.5-coder:7b</span></span>
        </div>
      </div>
    </div>
  );
}

function StatusBar() {
  return (
    <div className="statusbar">
      <span className="sb-item"><span className="ic"><Icon name="branch" size={11}/></span>main · 8a3f1d2</span>
      <span className="sb-item"><span className="ic"><Icon name="cpu" size={11}/></span>qwen2.5-coder:7b · 4.2GB</span>
      <span className="sb-item"><span className="ic"><Icon name="clock" size={11}/></span>completed 14:35:22</span>
      <span className="statusbar-spacer"/>
      <span className="sb-item accent">● 1 running</span>
      <span className="sb-item">3 queued</span>
      <span className="sb-item">14 done today</span>
      <span className="sb-item">tauri · 0.4.2</span>
    </div>
  );
}

/* ---------- Review header ---------- */
function ReviewHeader({ run }) {
  const sevs = [
    { k: "crit", l: "critical", v: run.severities.critical },
    { k: "high", l: "high",     v: run.severities.high },
    { k: "med",  l: "medium",   v: run.severities.medium },
    { k: "low",  l: "low",      v: run.severities.low },
    { k: "info", l: "info",     v: run.severities.info },
  ].filter(s => s.v > 0);

  return (
    <div className="review-header">
      <div className="rh-left">
        <div className="rh-title">
          <span className="ic"><Icon name={run.icon} size={16}/></span>
          {run.task}
          <span className="id">{run.id}</span>
        </div>
        <div className="rh-meta">
          <span><span className="k">project</span><span className="v">{run.project}</span></span>
          <span><span className="k">branch</span><span className="v">{run.branch}</span></span>
          <span><span className="k">commit</span><span className="v">{run.commit}</span></span>
          <span><span className="k">duration</span><span className="v">{run.duration}</span></span>
          <span><span className="k">files</span><span className="v">{run.files}</span></span>
          <span><span className="k">tokens</span><span className="v">{run.tokens}</span></span>
          <span><span className="k">model</span><span className="v">{run.model}</span></span>
        </div>
        <div className="sev-summary">
          {sevs.map(s => (
            <span key={s.k} className={`sev-chip ${s.k}`}>
              <span className="ct">{s.v}</span>
              {s.l}
            </span>
          ))}
        </div>
      </div>
      <div className="rh-actions">
        <button className="btn"><Icon name="refresh" size={13}/>Re-run</button>
        <div className="btn-group">
          <button className="btn" title="Export as Markdown"><Icon name="file" size={13}/>Markdown</button>
          <button className="btn" title="Export as JSON"><Icon name="code" size={13}/>JSON</button>
          <button className="btn" title="Copy to clipboard"><Icon name="layers" size={13}/>Copy</button>
        </div>
        <button className="btn btn-primary"><Icon name="check" size={13}/>Approve</button>
      </div>
    </div>
  );
}

/* ---------- Prompt list ---------- */
function PromptList({ activeId, onPick }) {
  return (
    <div className="prompt-list-pane">
      <div className="plp-head">
        <span className="t">Prompts</span>
        <span className="c">{R.prompts.length}</span>
        <div className="plp-head-spacer"/>
      </div>
      <div className="plp-list">
        {R.prompts.map((p, i) => {
          const type = detectOutputType(p);
          const ct = type === "structured" ? p.output.findings.length : 0;
          const isText = type === "text";
          return (
            <div
              key={p.id}
              className={`prompt-item${p.id === activeId ? " active" : ""}`}
              onClick={() => onPick(p.id)}
            >
              <div className="pi-num">{String(i + 1).padStart(2, "0")}</div>
              <div className="pi-body">
                <div className="pi-name">{p.name}</div>
                <div className="pi-meta">
                  <span>{p.duration}</span>
                  <span>·</span>
                  <span>{p.tokens}</span>
                  <span>·</span>
                  {isText ? (
                    <span className="findings-badge" style={{color:"var(--text-muted)", background:"var(--bg-elev)"}}>text</span>
                  ) : (
                    <span className={`findings-badge ${ct === 0 ? "zero" : "some"}`}>
                      {ct === 0 ? "clean" : `${ct} finding${ct === 1 ? "" : "s"}`}
                    </span>
                  )}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

/* ---------- Finding card ---------- */
const SEV_SHORT = { critical: "crit", high: "high", medium: "med", low: "low", info: "info" };
const KIND_LABEL = { bug: "BUG", fix: "FIX", suggestion: "SUGGESTION", idea: "IDEA", warning: "WARNING" };

function SevPill({ sev }) {
  const s = SEV_SHORT[sev] || "info";
  return <span className={`sev ${s}`}>{sev.toUpperCase()}</span>;
}

function FindingCard({ f }) {
  const s = SEV_SHORT[f.severity] || "info";
  return (
    <div className={`finding-card ${s}`}>
      <div className="fc-head">
        <div className="fc-badges">
          <SevPill sev={f.severity}/>
          <span className="kind-badge">{KIND_LABEL[f.kind] || f.kind.toUpperCase()}</span>
        </div>
        <div className="fc-title-wrap">
          <div className="fc-title">{f.title}</div>
          <div className={`fc-loc${f.location ? "" : " no-loc"}`}>
            {f.location ? (
              <>
                <Icon name="file" size={11}/>
                <span className="file">{f.location}</span>
              </>
            ) : (
              <>
                <Icon name="file" size={11}/>
                <span>no location reported</span>
              </>
            )}
          </div>
        </div>
        <div className="fc-actions">
          <button className="btn-icon" title="Open in editor"><Icon name="code" size={13}/></button>
          <button className="btn-icon" title="Copy"><Icon name="layers" size={13}/></button>
          <button className="btn-icon" title="More"><Icon name="more" size={13}/></button>
        </div>
      </div>
      <div className="fc-body">
        <div className="fc-desc">{f.description}</div>
        {f.suggested_fix && (
          <div className="fc-fix">
            <div className="label"><Icon name="wand" size={11}/>Suggested fix</div>
            <div className="text" dangerouslySetInnerHTML={{ __html: highlightCode(f.suggested_fix) }}/>
          </div>
        )}
        {f.tags && f.tags.length > 0 && (
          <div className="fc-tags">
            {f.tags.map(t => <span key={t} className="fc-tag">{t}</span>)}
          </div>
        )}
      </div>
    </div>
  );
}

// turn `code` into <code>code</code> in fix text
function highlightCode(text) {
  const esc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  return esc(text).replace(/`([^`]+)`/g, '<code>$1</code>');
}

/* ---------- Raw JSON view ---------- */
function RawJson({ data }) {
  const html = useMemo(() => syntaxHighlight(JSON.stringify(data, null, 2)), [data]);
  return <pre className="raw-json" dangerouslySetInnerHTML={{ __html: html }}/>;
}

function syntaxHighlight(json) {
  const esc = json.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  return esc.replace(
    /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(\.\d+)?([eE][+-]?\d+)?)/g,
    (match) => {
      let cls = "n"; // number
      if (/^"/.test(match)) {
        if (/:$/.test(match)) cls = "k";       // key
        else cls = "s";                        // string
      } else if (/true|false/.test(match)) {
        cls = "b";
      } else if (/null/.test(match)) {
        cls = "nl";
      }
      return `<span class="${cls}">${match}</span>`;
    }
  );
}

/* ---------- Raw text view ---------- */
function RawText({ text }) {
  return <pre className="raw-json" style={{whiteSpace:"pre-wrap"}}>{text}</pre>;
}

/* ---------- Minimal markdown renderer ---------- */
function renderMarkdown(src) {
  // Escape HTML first
  const esc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

  // Process inline formatting (after the line is escaped)
  const inline = (line) => {
    return line
      .replace(/`([^`]+)`/g, (_, c) => `<code>${c}</code>`)
      .replace(/\*\*([^*]+)\*\*/g, (_, c) => `<strong>${c}</strong>`)
      .replace(/(^|[^*])\*([^*\n]+)\*/g, (_, p, c) => `${p}<em>${c}</em>`)
      .replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_, t, h) => `<a href="${h}">${t}</a>`);
  };

  const lines = src.split("\n");
  const out = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];

    // Fenced code block
    if (/^```/.test(line)) {
      const buf = [];
      i++;
      while (i < lines.length && !/^```/.test(lines[i])) {
        buf.push(esc(lines[i]));
        i++;
      }
      i++; // closing fence
      out.push(`<pre><code>${buf.join("\n")}</code></pre>`);
      continue;
    }

    // Headings
    const h = line.match(/^(#{1,4})\s+(.*)$/);
    if (h) {
      const lvl = h[1].length;
      out.push(`<h${lvl}>${inline(esc(h[2]))}</h${lvl}>`);
      i++;
      continue;
    }

    // Horizontal rule
    if (/^---+\s*$/.test(line)) {
      out.push("<hr/>");
      i++;
      continue;
    }

    // Blockquote (consume consecutive >)
    if (/^>\s?/.test(line)) {
      const buf = [];
      while (i < lines.length && /^>\s?/.test(lines[i])) {
        buf.push(lines[i].replace(/^>\s?/, ""));
        i++;
      }
      out.push(`<blockquote>${renderMarkdown(buf.join("\n"))}</blockquote>`);
      continue;
    }

    // Unordered list (consume consecutive - or *)
    if (/^[-*]\s+/.test(line)) {
      const buf = [];
      while (i < lines.length && /^[-*]\s+/.test(lines[i])) {
        buf.push(`<li>${inline(esc(lines[i].replace(/^[-*]\s+/, "")))}</li>`);
        i++;
      }
      out.push(`<ul>${buf.join("")}</ul>`);
      continue;
    }

    // Ordered list
    if (/^\d+\.\s+/.test(line)) {
      const buf = [];
      while (i < lines.length && /^\d+\.\s+/.test(lines[i])) {
        buf.push(`<li>${inline(esc(lines[i].replace(/^\d+\.\s+/, "")))}</li>`);
        i++;
      }
      out.push(`<ol>${buf.join("")}</ol>`);
      continue;
    }

    // Blank line → paragraph break
    if (/^\s*$/.test(line)) {
      i++;
      continue;
    }

    // Paragraph (consume consecutive non-blank non-special lines)
    const buf = [];
    while (
      i < lines.length &&
      !/^\s*$/.test(lines[i]) &&
      !/^(#{1,4}\s|```|>\s?|[-*]\s+|\d+\.\s+|---+\s*$)/.test(lines[i])
    ) {
      buf.push(inline(esc(lines[i])));
      i++;
    }
    if (buf.length) out.push(`<p>${buf.join(" ")}</p>`);
  }

  return out.join("\n");
}

function TextOutput({ text }) {
  const html = useMemo(() => renderMarkdown(text), [text]);
  return <div className="text-output" dangerouslySetInnerHTML={{ __html: html }}/>;
}

/* ---------- Output type detection ---------- */
function detectOutputType(prompt) {
  if (prompt.outputType) return prompt.outputType;
  if (typeof prompt.output === "string") return "text";
  if (prompt.output && Array.isArray(prompt.output.findings)) return "structured";
  return "text";
}

/* ---------- Result pane ---------- */
function ResultPane({ promptId }) {
  const prompt = R.prompts.find(p => p.id === promptId) || R.prompts[0];
  const type = detectOutputType(prompt);
  const isText = type === "text";
  const [tab, setTab] = useState("default");
  const [filter, setFilter] = useState("all");

  // Findings (empty for text outputs) — always called so hook count is stable
  const findings = isText ? [] : (prompt.output.findings || []);
  const sevCounts = useMemo(() => {
    const c = { critical: 0, high: 0, medium: 0, low: 0, info: 0 };
    findings.forEach(f => { c[f.severity] = (c[f.severity] || 0) + 1; });
    return c;
  }, [findings]);

  if (isText) {
    const showRaw = tab === "raw";
    return (
      <div className="result-pane">
        <div className="result-pane-head">
          <h3>{prompt.name}</h3>
          <div className="rph-meta">
            <span><span className="k">duration</span>{prompt.duration}</span>
            <span><span className="k">tokens</span>{prompt.tokens}</span>
            <span className="type-chip text"><Icon name="file" size={9}/>plaintext</span>
          </div>
          <div className="rph-spacer"/>
          <div className="tabs">
            <button className={`tab${!showRaw ? " active" : ""}`} onClick={() => setTab("default")}>Rendered</button>
            <button className={`tab${showRaw ? " active" : ""}`} onClick={() => setTab("raw")}>Raw</button>
          </div>
        </div>

        <div className="schema-warn">
          <span className="ic"><Icon name="bulb" size={14}/></span>
          <div className="body">
            <div className="t">Unstructured output</div>
            <div className="s">This prompt did not return JSON matching the findings schema. Displaying as plaintext — findings filters and counts are unavailable.</div>
          </div>
        </div>

        {showRaw
          ? <RawText text={prompt.output}/>
          : <TextOutput text={prompt.output}/>
        }
      </div>
    );
  }

  // Structured path
  const showRaw = tab === "raw";
  const visible = filter === "all"
    ? findings
    : findings.filter(f => f.severity === filter);

  return (
    <div className="result-pane">
      <div className="result-pane-head">
        <h3>{prompt.name}</h3>
        <div className="rph-meta">
          <span><span className="k">duration</span>{prompt.duration}</span>
          <span><span className="k">tokens</span>{prompt.tokens}</span>
          <span><span className="k">findings</span>{findings.length}</span>
          <span className="type-chip structured"><Icon name="check" size={9}/>structured</span>
        </div>
        <div className="rph-spacer"/>
        <div className="tabs">
          <button className={`tab${!showRaw ? " active" : ""}`} onClick={() => setTab("default")}>Structured</button>
          <button className={`tab${showRaw ? " active" : ""}`} onClick={() => setTab("raw")}>Raw JSON</button>
        </div>
      </div>

      {!showRaw ? (
        <>
          <div className="summary-card">
            <span className="sc-ic"><Icon name="sparkles" size={16}/></span>
            <div className="sc-body">
              <div className="sc-label">Summary</div>
              <div className="sc-text">{prompt.output.summary}</div>
            </div>
          </div>

          {findings.length > 0 ? (
            <>
              <div className="filter-row">
                <button
                  className={`filter-chip${filter === "all" ? " active" : ""}`}
                  onClick={() => setFilter("all")}
                >All <span className="ct">{findings.length}</span></button>
                {["critical","high","medium","low","info"].map(sev =>
                  sevCounts[sev] > 0 && (
                    <button
                      key={sev}
                      className={`filter-chip${filter === sev ? " active" : ""}`}
                      onClick={() => setFilter(sev)}
                    >
                      {sev} <span className="ct">{sevCounts[sev]}</span>
                    </button>
                  )
                )}
                <div className="grow"/>
                <span className="filter-chip" style={{cursor:"default"}}>
                  showing <span className="ct">{visible.length}</span>
                </span>
              </div>

              <div className="findings-section-head">
                <h4>Findings</h4>
                <span className="count">{visible.length} of {findings.length}</span>
              </div>

              {visible.map((f, i) => <FindingCard key={i} f={f}/>)}
            </>
          ) : (
            <div className="no-findings">
              <div className="ic"><Icon name="check" size={20}/></div>
              <div className="t">No findings reported</div>
              <div className="s">This prompt completed cleanly — nothing actionable surfaced.</div>
            </div>
          )}
        </>
      ) : (
        <RawJson data={prompt.output}/>
      )}
    </div>
  );
}

/* ---------- App ---------- */
function App() {
  const [activePromptId, setActivePromptId] = useState(R.activePromptId);
  const [activeRunId, setActiveRunId] = useState("r-014");

  return (
    <>
      <Titlebar/>
      <div className="body">
        <ActivityBar/>
        <RecentSidebar activeId={activeRunId} onPick={setActiveRunId}/>
        <div className="main">
          <ReviewHeader run={R.run}/>
          <div className="review-body">
            <PromptList activeId={activePromptId} onPick={setActivePromptId}/>
            <ResultPane promptId={activePromptId}/>
          </div>
        </div>
      </div>
      <StatusBar/>
    </>
  );
}

ReactDOM.createRoot(document.getElementById("app")).render(<App/>);
