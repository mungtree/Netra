// Chatur — Local LLM AI Manager
const { useState } = React;
const D = window.CHATUR_DATA;

/* ---------- Titlebar ---------- */
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

/* ---------- Activity bar ---------- */
function ActivityBar() {
  const items = [
    { id: "projects", icon: "folder", active: true },
    { id: "library",  icon: "library" },
    { id: "history",  icon: "clock" },
    { id: "activity", icon: "activity" },
  ];
  return (
    <div className="activitybar">
      {items.map(i => (
        <button key={i.id} className={`ab-btn${i.active ? " active" : ""}`} title={i.id}>
          <Icon name={i.icon}/>
        </button>
      ))}
      <div className="ab-spacer"/>
      <button className="ab-btn" title="Settings"><Icon name="settings"/></button>
    </div>
  );
}

/* ---------- Sidebar ---------- */
function Sidebar({ activeId, onPick }) {
  return (
    <div className="sidebar">
      <div className="sb-header">
        <div className="sb-title">Projects</div>
        <button className="sb-add" title="Add project"><Icon name="plus" size={14}/></button>
      </div>
      <div className="sb-search">
        <Icon name="search" size={13}/>
        <input placeholder="Search projects…"/>
        <span className="kbd">⌘P</span>
      </div>
      <div className="proj-list">
        {D.projects.map(p => (
          <div
            key={p.id}
            className={`proj-item${p.id === activeId ? " active" : ""}`}
            onClick={() => onPick(p.id)}
          >
            <span className={`proj-status ${p.status}`}/>
            <div className="proj-info">
              <div className="proj-name">{p.name}</div>
              <div className="proj-path">{p.path}</div>
            </div>
            <div className="proj-count">{p.count || ""}</div>
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

/* ---------- Main: header + wizard + last run ---------- */
function MainHeader({ project }) {
  return (
    <div className="main-header">
      <div className="breadcrumb">
        <Icon name="folder" size={13}/>
        <span>~/code</span>
        <span className="sep">/</span>
        <span className="cur">{project.name}</span>
        <span className="branch">
          <Icon name="branch" size={10}/>
          main
        </span>
      </div>
      <div className="main-header-spacer"/>
      <div className="main-header-meta">
        <span><span className="k">files</span><span className="v">218</span></span>
        <span><span className="k">lines</span><span className="v">42.1k</span></span>
        <span><span className="k">lang</span><span className="v">TypeScript · Vue · Rust</span></span>
      </div>
    </div>
  );
}

function TaskGrid() {
  return (
    <>
      <div className="wizard-head">
        <h2><span className="step">02</span>Pick a task to queue</h2>
        <span className="hint">⌘ + 1–8 to queue · ⇧⌘ + 1–8 to run now</span>
      </div>
      <div className="task-grid">
        {D.tasks.map(t => (
          <button key={t.id} className={`task-card${t.featured ? " featured" : ""}`}>
            <span className="tc-shortcut">⌘{t.shortcut}</span>
            <div className="tc-icon"><Icon name={t.icon} size={16}/></div>
            <div className="tc-title">{t.title}</div>
            <div className="tc-desc">{t.desc}</div>
            <div className="tc-meta">{t.meta}</div>
          </button>
        ))}
      </div>
    </>
  );
}

function LastRun() {
  const r = D.lastRun;
  return (
    <div className="run-block">
      <div className="run-block-head">
        <div className="rb-title">
          <span className="ic"><Icon name={r.icon} size={14}/></span>
          Last run · {r.task}
        </div>
        <div className="rb-meta">
          <span><span className="k">finished</span>{r.finishedAt}</span>
          <span><span className="k">took</span>{r.duration}</span>
          <span><span className="k">files</span>{r.files}</span>
          <span><span className="k">tokens</span>{r.tokens}</span>
          <span><span className="k">findings</span>{r.findings.length}</span>
        </div>
      </div>
      <div className="findings">
        {r.findings.map((f, i) => (
          <div key={i} className="finding">
            <span className={`sev ${f.sev}`}>{f.sev.toUpperCase()}</span>
            <div className="f-body">
              <div className="f-msg">{f.msg}</div>
              <div className="f-loc">
                <span className="file">{f.file}</span>
                <span className="line"> · {f.range}</span>
              </div>
            </div>
            <div className="f-actions">
              <button className="btn-icon" title="Open in editor"><Icon name="code" size={13}/></button>
              <button className="btn-icon" title="Re-run on this file"><Icon name="refresh" size={13}/></button>
              <button className="btn-icon" title="More"><Icon name="more" size={13}/></button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

/* ---------- Queue panel ---------- */
function QueuePanel() {
  const q = D.queue;
  return (
    <div className="queue">
      <div className="queue-head">
        <div className="q-title">Queue</div>
        <div className="q-count">{q.stats.running + q.stats.pending} active</div>
        <div className="queue-head-spacer"/>
        <button className="q-ctrl" title="Pause queue"><Icon name="pause" size={14}/></button>
        <button className="q-ctrl" title="Clear completed"><Icon name="x" size={14}/></button>
      </div>

      <div className="queue-scroll">
        {/* Running */}
        <div className="q-group">Running<span className="line"/></div>
        <div className="q-item running">
          <div className="q-item-head">
            <span className="q-ic"><Icon name={q.running.icon} size={13}/></span>
            <span className="q-name">{q.running.task}</span>
            <span className="q-time">{q.running.eta}</span>
          </div>
          <div className="q-item-sub">
            <span className="repo">{q.running.project}</span>
            <span>·</span>
            <span>{q.running.step}</span>
          </div>
          <div className="q-progress"><div className="bar"/></div>
          <div className="q-progress-meta">
            <span>prompt {q.running.promptIdx}/{q.running.promptTotal}</span>
            <span>{Math.round(q.running.progress * 100)}%</span>
          </div>
          <div className="q-actions">
            <button className="btn-mini"><Icon name="pause" size={10}/>Pause</button>
            <button className="btn-mini danger"><Icon name="x" size={10}/>Cancel</button>
          </div>
        </div>

        {/* Pending */}
        <div className="q-group">Pending · {q.pending.length}<span className="line"/></div>
        {q.pending.map((p, i) => (
          <div key={i} className="q-item">
            <div className="q-item-head">
              <span className="q-ic"><Icon name={p.icon} size={13}/></span>
              <span className="q-name">{p.task}</span>
              <span className="q-time">#{i + 2}</span>
            </div>
            <div className="q-item-sub">
              <span className="repo">{p.project}</span>
              <span>·</span>
              <span>{p.prompts} prompts</span>
            </div>
          </div>
        ))}

        {/* Done */}
        <div className="q-group">Completed · today<span className="line"/></div>
        {q.done.map((p, i) => (
          <div key={i} className="q-item done">
            <div className="q-item-head">
              <span className="q-ic"><Icon name="check" size={13}/></span>
              <span className="q-name">{p.task}</span>
              <span className="q-time">{p.duration}</span>
            </div>
            <div className="q-item-sub">
              <span className="repo">{p.project}</span>
              <span>·</span>
              <span className={`findings${p.highSev > 0 ? " high" : ""}`}>
                {p.findings} findings{p.highSev > 0 ? ` · ${p.highSev} high` : ""}
              </span>
            </div>
          </div>
        ))}
      </div>

      <div className="queue-foot">
        <div className="q-stat">
          <span className="label">Running</span>
          <span className="val run">{q.stats.running}</span>
        </div>
        <div className="q-stat">
          <span className="label">Pending</span>
          <span className="val">{q.stats.pending}</span>
        </div>
        <div className="q-stat">
          <span className="label">Done</span>
          <span className="val done">{q.stats.done}</span>
        </div>
      </div>
    </div>
  );
}

/* ---------- Status bar ---------- */
function StatusBar() {
  return (
    <div className="statusbar">
      <span className="sb-item"><span className="ic"><Icon name="branch" size={11}/></span>main</span>
      <span className="sb-item"><span className="ic"><Icon name="cpu" size={11}/></span>qwen2.5-coder:7b · 4.2GB</span>
      <span className="sb-item"><span className="ic"><Icon name="activity" size={11}/></span>GPU 71% · 12.4 tok/s</span>
      <span className="statusbar-spacer"/>
      <span className="sb-item accent">● 1 running</span>
      <span className="sb-item">3 queued</span>
      <span className="sb-item">14 done today</span>
      <span className="sb-item">tauri · 0.4.2</span>
    </div>
  );
}

/* ---------- App ---------- */
function App() {
  const [activeId, setActiveId] = useState(D.activeProjectId);
  const project = D.projects.find(p => p.id === activeId) || D.projects[0];

  return (
    <>
      <Titlebar/>
      <div className="body">
        <ActivityBar/>
        <Sidebar activeId={activeId} onPick={setActiveId}/>
        <div className="main">
          <MainHeader project={project}/>
          <div className="main-scroll">
            <TaskGrid/>
            <LastRun/>
          </div>
        </div>
        <QueuePanel/>
      </div>
      <StatusBar/>
    </>
  );
}

ReactDOM.createRoot(document.getElementById("app")).render(<App/>);
