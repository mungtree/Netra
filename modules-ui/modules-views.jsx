// modules-views.jsx
// The other surfaces affected by the module concept:
//   - ProjectsOverview         (new top-level page)
//   - BatchBuilder             (modal, Global ON and OFF variants)
//   - QueueWithModules         (queue panel showing module badge on each row)
//   - ResumeBanner             (startup-resume banner, subtle in main pane)

const M3 = window.MODULES_DATA;

/* ===================================================================
   Projects & Modules overview
   =================================================================== */
function ProjectsOverview() {
  const allMods = M3.projects.flatMap(p => p.modules);
  const filesTotal = M3.projects.reduce((n,p)=>n+p.filesTotal,0);

  return (
    <>
      <div className="po-head">
        <div>
          <h2>Projects & Modules</h2>
          <div className="sub">
            All projects connected to Chatur. Click any project to manage its
            modules in the Modules tab.
          </div>
        </div>
        <div className="actions">
          <div className="po-search">
            <Icon name="search" size={12}/>
            <input placeholder="Filter projects or modules…" readOnly/>
            <span className="kbd">⌘F</span>
          </div>
          <button className="btn"><Icon name="upload" size={12}/>Import</button>
          <button className="btn-primary"><Icon name="plus" size={12}/>Add project</button>
        </div>
      </div>

      <div className="po-body">
        <div className="po-stats">
          <div className="po-stat">
            <div className="label">Projects</div>
            <div className="val">{M3.projects.length}<span className="unit">connected</span></div>
          </div>
          <div className="po-stat">
            <div className="label">Modules</div>
            <div className="val accent">{allMods.length}<span className="unit">across repos</span></div>
          </div>
          <div className="po-stat">
            <div className="label">Files indexed</div>
            <div className="val">{filesTotal.toLocaleString()}</div>
          </div>
          <div className="po-stat">
            <div className="label">Avg scope</div>
            <div className="val">{Math.round(filesTotal / allMods.length)}<span className="unit">files / module</span></div>
          </div>
        </div>

        <div className="po-section-head">
          <h3>All projects</h3>
          <span className="hint">{M3.projects.length} · sorted by recent activity</span>
        </div>

        <div className="po-table">
          {M3.projects.map(p => {
            const isDefault = p.modules.length === 1 && /^(root)$/i.test(p.modules[0].name);
            return (
              <div key={p.id} className="po-row">
                <span className={`stat-dot ${p.status}`}/>
                <div className="pname">
                  <span>{p.name}</span>
                  <span className="ppath">{p.path}</span>
                </div>
                <div className="stack">
                  {p.id === "chatur"     && <><span className="tag">TS</span><span className="tag">Svelte</span><span className="tag">Rust</span><span className="tag">Tauri</span></>}
                  {p.id === "linear-x"   && <><span className="tag">TS</span><span className="tag">React</span></>}
                  {p.id === "rust-fork"  && <><span className="tag">Rust</span></>}
                  {p.id === "portfolio"  && <><span className="tag">Astro</span><span className="tag">MD</span></>}
                  {p.id === "ml-nb"      && <><span className="tag">Python</span><span className="tag">.ipynb</span></>}
                </div>
                <div className="modcount">
                  <span className="label">MODULES</span>
                  <span style={{color: p.modules.length > 1 ? "var(--accent)" : "var(--text-muted)"}}>{p.modules.length}</span>
                  {isDefault && <span style={{
                    fontSize:10, color:"var(--text-dim)",
                    marginLeft:6, fontFamily:"var(--font-mono)"
                  }}>default</span>}
                </div>
                <div className="modchips">
                  {p.modules.slice(0,6).map(m => (
                    <span key={m.id} className={`modchip${isDefault ? " default" : ""}`}>{m.name}</span>
                  ))}
                  {p.modules.length > 6 && <span className="modchip muted">+{p.modules.length - 6}</span>}
                </div>
                <div className="row-actions">
                  <button className="btn-icon" title="Open modules"><Icon name="layers" size={12}/></button>
                  <button className="btn-icon" title="Infer with AI"><Icon name="sparkles" size={12}/></button>
                  <button className="btn-icon" title="More"><Icon name="more" size={12}/></button>
                </div>
              </div>
            );
          })}
        </div>

        <div style={{
          marginTop: 14, fontSize: 11, color: "var(--text-dim)",
          fontFamily: "var(--font-mono)",
        }}>
          tip · projects with one default module fan out as prompts × targets;
          adding more modules splits each job into smaller, focused contexts.
        </div>
      </div>
    </>
  );
}

/* ===================================================================
   Batch builder modal
   =================================================================== */
function PromptChip({ children }) {
  return (
    <span className="modchip" style={{borderColor:"var(--border-strong)", background:"var(--bg)"}}>
      {children}
    </span>
  );
}

function BatchBuilder({ globalMode = false }) {
  const batch = M3.batch;
  const prompts = batch.promptIds.map(id => {
    const meta = {
      bugs:  { name: "Find Bugs",            count: 47 },
      vulns: { name: "Find Vulnerabilities", count: 38 },
    };
    return meta[id];
  });
  const targets = batch.targetProjectIds.map(id => M3.projects.find(p => p.id === id));

  const promptsCount  = prompts.reduce((n,p)=>n+p.count,0);
  const modulesCount  = targets.reduce((n,t)=>n + (batch.moduleSelection[t.id]?.length||0), 0);
  const totalJobs     = globalMode
    ? promptsCount * targets.length
    : targets.reduce((n,t)=>n + promptsCount * (batch.moduleSelection[t.id]?.length||0), 0) / prompts.length * prompts.length;

  // Cleaner per-target × per-prompt math:
  const jobs = globalMode
    ? prompts.length * targets.length
    : targets.reduce((n,t)=>n + prompts.length * (batch.moduleSelection[t.id]?.length||0), 0);
  const promptN = prompts.length;
  const targetN = targets.length;
  const modN    = targets.reduce((n,t)=>n+(batch.moduleSelection[t.id]?.length||0), 0);

  return (
    <div className="batch-overlay">
      <div className="batch-modal">
        <div className="batch-head">
          <Icon name="layers" size={14} className="" />
          <div>
            <div className="title">New batch</div>
            <div className="sub">2 prompts · {targetN} target{targetN!==1?"s":""}</div>
          </div>
          <button className="x"><Icon name="x" size={14}/></button>
        </div>

        <div className="batch-body">
          {/* Prompts row (compact summary) */}
          <div className="batch-section">
            <div className="sec-head">
              <h4>Prompts</h4>
              <span className="hint">{promptN} selected · {promptsCount} total invocations</span>
            </div>
            <div style={{display:"flex", gap:6, flexWrap:"wrap"}}>
              {prompts.map((p,i) => (
                <PromptChip key={i}>{p.name} <span style={{color:"var(--text-dim)"}}>· {p.count}</span></PromptChip>
              ))}
              <button className="modchip" style={{cursor:"pointer", color:"var(--text-muted)"}}>
                <span style={{color:"var(--accent)"}}>+</span> add prompt
              </button>
            </div>
          </div>

          {/* Targets row (compact summary) */}
          <div className="batch-section">
            <div className="sec-head">
              <h4>Targets</h4>
              <span className="hint">{targetN} project{targetN!==1?"s":""} selected</span>
            </div>
            <div style={{display:"flex", gap:6, flexWrap:"wrap"}}>
              {targets.map(t => (
                <PromptChip key={t.id}>{t.name} <span style={{color:"var(--text-dim)"}}>· {t.modules.length} mod</span></PromptChip>
              ))}
              <button className="modchip" style={{cursor:"pointer", color:"var(--text-muted)"}}>
                <span style={{color:"var(--accent)"}}>+</span> add target
              </button>
            </div>
          </div>

          {/* Global toggle */}
          <div className={`global-row${globalMode ? " on" : ""}`}>
            <Icon name="package" size={16} className=""/>
            <div className="body">
              <div className="title">Global (skip modules)</div>
              <div className="desc">
                {globalMode
                  ? <>On — each target is scanned in full. Fanout collapses to <code>prompts × targets</code>.</>
                  : <>Off — each agent gets a single module's scope. Fanout is <code>prompts × targets × modules</code>.</>}
              </div>
            </div>
            <div className={`toggle${globalMode ? " on" : ""}`}/>
          </div>

          {/* Per-target module picker (dimmed when global on) */}
          <div className={`batch-section${globalMode ? " disabled" : ""}`}>
            <div className="sec-head">
              <h4>Per-target modules</h4>
              <span className="hint">
                {globalMode
                  ? "ignored while Global is on"
                  : <>tap a chip to exclude · default = all modules selected</>}
              </span>
              {globalMode && <span className="disabled-tag">SKIPPED</span>}
            </div>
            {targets.map(t => {
              const selected = batch.moduleSelection[t.id] || [];
              return (
                <div key={t.id} className="target-row">
                  <span className={`stat-dot ${t.status}`}/>
                  <div className="pname">
                    <span>{t.name}</span>
                    <span className="path">{t.path}</span>
                  </div>
                  <div className="modchips" style={{display:"flex", gap:5, flexWrap:"wrap"}}>
                    {t.modules.map(m => {
                      const active = selected.includes(m.id);
                      return (
                        <span key={m.id} className={`modchip sel${active ? " active" : ""}`}>
                          {m.name}
                          {active && <Icon name="x" size={9} className="x"/>}
                        </span>
                      );
                    })}
                  </div>
                  <div className="modcount-mini">
                    <b>{selected.length}</b><span style={{color:"var(--text-dim)"}}>/{t.modules.length}</span>
                    <div style={{fontSize:9, color:"var(--text-dim)"}}>selected</div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        <div className="batch-foot">
          <div className="job-preview">
            <span className="n">{jobs}</span>jobs will be queued
            <div className="formula">
              = <b>{promptN}</b> prompts × <b>{targetN}</b> {targetN===1?"project":"projects"}
              {!globalMode && <> × <b>{modN}</b> module{modN!==1?"s":""}</>}
            </div>
          </div>
          <div className="spacer"/>
          <button className="btn">Save as preset</button>
          <button className="btn">Cancel</button>
          <button className="btn-go">
            <Icon name="play" size={11}/>Queue {jobs} job{jobs===1?"":"s"}
            <span className="kbd" style={{marginLeft:4, color:"#0a0a0a", borderColor:"rgba(0,0,0,0.3)", background:"rgba(0,0,0,0.1)"}}>⇧⏎</span>
          </button>
        </div>
      </div>
    </div>
  );
}

/* ===================================================================
   Queue panel with module badges
   =================================================================== */
function QueueWithModules() {
  const q = M3.queueWithModules;
  return (
    <div className="queue">
      <div className="queue-head">
        <div className="q-title">Queue</div>
        <div className="q-count">{q.stats.running + q.stats.pending} active</div>
        <div className="queue-head-spacer"/>
        <button className="q-ctrl"><Icon name="pause" size={14}/></button>
        <button className="q-ctrl"><Icon name="x" size={14}/></button>
      </div>

      <div className="queue-scroll">
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
          <div className="q-modline">
            <span className="ic"><Icon name="layers" size={10}/></span>
            <span className="modname">{q.running.module}</span>
            <span>·</span>
            <span className="promptcount">prompt {q.running.promptIdx}/{q.running.promptTotal}</span>
          </div>
          <div className="q-progress"><div className="bar"/></div>
          <div className="q-progress-meta">
            <span>{Math.round(q.running.progress*100)}%</span>
            <span>·</span>
          </div>
        </div>

        <div className="q-group">Pending · {q.pending.length}<span className="line"/></div>
        {q.pending.map((p, i) => (
          <div key={i} className="q-item">
            <div className="q-item-head">
              <span className="q-ic"><Icon name={p.icon} size={13}/></span>
              <span className="q-name">{p.task}</span>
              <span className="q-time">#{i+2}</span>
            </div>
            <div className="q-item-sub">
              <span className="repo">{p.project}</span>
              <span>·</span>
              <span>{p.prompts} prompts</span>
            </div>
            <div className="q-modline">
              <span className="ic"><Icon name="layers" size={10}/></span>
              <span className="modname">{p.module}</span>
            </div>
          </div>
        ))}

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
              <span className={`findings${p.highSev>0 ? " high" : ""}`}>
                {p.findings} findings{p.highSev>0 ? ` · ${p.highSev} high` : ""}
              </span>
            </div>
            <div className="q-modline">
              <span className="ic"><Icon name="layers" size={10}/></span>
              <span className="modname">{p.module}</span>
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

/* ===================================================================
   Resume banner
   =================================================================== */
function ResumeBanner() {
  const r = M3.resume;
  return (
    <div className="resume-banner">
      <span className="ic"><Icon name="refresh" size={14}/></span>
      <div className="msg">
        Resumed <b>{r.queuedAtRestart} queued job{r.queuedAtRestart!==1?"s":""}</b> from your last session
        <span className="meta">restored {r.resumedAt} · {r.discarded} cancelled (module deleted)</span>
      </div>
      <button className="btn-banner">Review</button>
      <button className="dismiss"><Icon name="x" size={12}/></button>
    </div>
  );
}

window.ProjectsOverview = ProjectsOverview;
window.BatchBuilder = BatchBuilder;
window.QueueWithModules = QueueWithModules;
window.ResumeBanner = ResumeBanner;
