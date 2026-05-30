// modules-editor.jsx
// Modules tab inside SettingsPane. One <ModulesPane state="..."/> covers
// every state needed for the design canvas.
//
// States:
//   empty       — only the default "root" module + onboarding hint
//   populated   — typical state with 5 well-scoped modules
//   adding      — inline "new module" row open
//   validation  — populated + a row showing invalid path + overlap warning
//   inferLoading — AI inference spinner with steps
//   inferDiff   — accept/reject preview comparing current vs proposed

const M2 = window.MODULES_DATA;
const chaturProj = M2.projects.find(p => p.id === "chatur");
const portfolioProj = M2.projects.find(p => p.id === "portfolio");

/* ---------- Shared bits ---------- */
function ScopePill({ project }) {
  return (
    <span className="scope-pill">
      <span className="ic"><Icon name="folder" size={11}/></span>
      <span>{project.name}</span>
      <span className="path">· {project.path}</span>
    </span>
  );
}

function ModuleListHeader({ count, onInfer = true }) {
  return (
    <div className="mod-toolbar">
      <div className="left">
        <strong>{count}</strong> module{count !== 1 ? "s" : ""} ·{" "}
        fanout = <strong>prompts × targets × modules</strong>
      </div>
      <div className="spacer"/>
      <button className="btn"><Icon name="upload" size={12}/>Import</button>
      <button className="btn"><Icon name="plus" size={12}/>Add module<span className="kbd">⌘N</span></button>
      {onInfer && (
        <button className="btn-infer">
          <span className="ic"><Icon name="sparkles" size={13}/></span>
          Infer with AI
        </button>
      )}
    </div>
  );
}

/* ---------- Row: read-only ---------- */
function ModRow({ mod, isDefault = false, warn = null }) {
  return (
    <div className={`mod-row${isDefault ? " default" : ""}`}>
      <span className="ic"><Icon name={isDefault ? "package" : "layers"} size={13}/></span>
      <div className="name"><span>{mod.name}</span></div>
      <div className="desc">{mod.desc}</div>
      <div className="path">
        <Icon name="folder" size={11}/>
        <span>{mod.root}</span>
        <span className="files">{mod.files} files · {mod.lines}</span>
      </div>
      <div className="row-actions">
        <button className="btn-icon" title="Edit"><Icon name="code" size={12}/></button>
        <button className="btn-icon" title="Duplicate"><Icon name="copy" size={12}/></button>
        <button className="btn-icon danger" title="Delete" disabled={isDefault}><Icon name="trash" size={12}/></button>
      </div>
      {warn && (
        <div className={`mod-warn${warn.kind === "err" ? " err" : ""}`}>
          <span className="ic"><Icon name="info" size={12}/></span>
          <div className="body">{warn.body}</div>
        </div>
      )}
    </div>
  );
}

/* ---------- Row: editing / adding ---------- */
function ModRowEditing({
  name = "", desc = "", root = "",
  placeholders = {},
  isNew = false,
}) {
  return (
    <div className="mod-row editing">
      <span className="ic"><Icon name={isNew ? "plus" : "code"} size={13}/></span>
      <div className="name">
        <input
          className="cell-input mono"
          defaultValue={name}
          placeholder={placeholders.name || "name"}
        />
      </div>
      <div className="desc">
        <input
          className="cell-input"
          defaultValue={desc}
          placeholder={placeholders.desc || "one-line description"}
        />
      </div>
      <div className="path" style={{display:"flex", alignItems:"center"}}>
        <input
          className="cell-input mono"
          defaultValue={root}
          placeholder={placeholders.root || "subdir/relative/path"}
          style={{flex:1}}
        />
        <button className="pick-btn" title="Pick folder">
          <Icon name="folder" size={11}/>Pick
        </button>
      </div>
      <div className="row-actions">
        <button className="btn-icon commit" title="Save"><Icon name="check" size={12}/></button>
        <button className="btn-icon" title="Cancel"><Icon name="x" size={12}/></button>
      </div>
    </div>
  );
}

/* ---------- Empty-hint card (shown beneath the default-only list) ---------- */
function EmptyHint() {
  return (
    <div className="mod-empty-hint">
      <span className="ic"><Icon name="layers" size={20}/></span>
      <div className="body">
        <b>Every project has at least one module.</b> Newly-added projects start
        with a single <code style={{fontFamily:"var(--font-mono)", color:"var(--accent-soft)"}}>root</code> module
        covering the whole repo. Split it into smaller scopes so agents stay
        focused — or let Chatur infer them.
      </div>
      <div className="actions">
        <button className="btn"><Icon name="plus" size={12}/>Add manually</button>
        <button className="btn-infer"><Icon name="sparkles" size={13}/>Infer with AI</button>
      </div>
    </div>
  );
}

/* ---------- ModulesPane variants ---------- */

/* Empty / default — just the seeded root row + onboarding hint */
function PaneEmpty() {
  const root = portfolioProj.modules[0];
  return (
    <div className="settings-pane">
      <div className="settings-head">
        <div>
          <h2>Modules</h2>
          <div className="sub">
            Split a project into named subdirectory scopes so each agent gets a
            tighter context window. Fanout = prompts × targets × modules.
          </div>
        </div>
        <div className="settings-head-actions">
          <ScopePill project={portfolioProj}/>
        </div>
      </div>
      <div className="settings-body">
        <ModuleListHeader count={1}/>
        <div className="mod-list">
          <ModRow mod={root} isDefault/>
          <div className="mod-add-row">
            <span className="ic"><Icon name="plus" size={13}/></span>
            <span>Add module… <span className="kbd" style={{marginLeft:6}}>⌘N</span></span>
          </div>
        </div>
        <EmptyHint/>
      </div>
    </div>
  );
}

/* Populated — typical 5-module setup for chatur */
function PanePopulated() {
  return (
    <div className="settings-pane">
      <div className="settings-head">
        <div>
          <h2>Modules</h2>
          <div className="sub">
            Split a project into named subdirectory scopes so each agent gets a
            tighter context window. Fanout = prompts × targets × modules.
          </div>
        </div>
        <div className="settings-head-actions">
          <ScopePill project={chaturProj}/>
        </div>
      </div>
      <div className="settings-body">
        <ModuleListHeader count={chaturProj.modules.length}/>
        <div className="mod-list">
          {chaturProj.modules.map(m => <ModRow key={m.id} mod={m}/>)}
          <div className="mod-add-row">
            <span className="ic"><Icon name="plus" size={13}/></span>
            <span>Add module…</span>
          </div>
        </div>
      </div>
    </div>
  );
}

/* Adding inline — last row is an editing input row */
function PaneAdding() {
  return (
    <div className="settings-pane">
      <div className="settings-head">
        <div>
          <h2>Modules</h2>
          <div className="sub">Adding a new module to <b style={{color:"var(--text)"}}>chatur</b>. Path is relative to the project root.</div>
        </div>
        <div className="settings-head-actions">
          <ScopePill project={chaturProj}/>
        </div>
      </div>
      <div className="settings-body">
        <ModuleListHeader count={chaturProj.modules.length}/>
        <div className="mod-list">
          {chaturProj.modules.map(m => <ModRow key={m.id} mod={m}/>)}
          <ModRowEditing
            isNew
            name="models"
            desc="Model providers, prompts, tool policy"
            root="src-tauri/src/models"
          />
        </div>
        <div style={{
          marginTop: 10, fontSize: 11, color: "var(--text-muted)",
          fontFamily: "var(--font-mono)",
          display: "flex", gap: 14,
        }}>
          <span><span className="kbd">⏎</span> save</span>
          <span><span className="kbd">esc</span> cancel</span>
          <span><span className="kbd">⌘D</span> pick folder…</span>
        </div>
      </div>
    </div>
  );
}

/* Validation — populated + two rows with warnings (invalid path, overlap) */
function PaneValidation() {
  // Spread a couple of synthetic warnings onto specific rows
  const rows = [
    { mod: chaturProj.modules[0], warn: null },
    { mod: { ...chaturProj.modules[1], root: "src-tauri/src/cmds" }, warn: {
      kind: "err",
      body: (
        <span>
          <b>Folder does not exist.</b>{" "}
          <em>`src-tauri/src/cmds` was not found inside the project root. Pick another folder or create it on disk.</em>
        </span>
      )
    }},
    { mod: chaturProj.modules[2], warn: null },
    { mod: chaturProj.modules[3], warn: null },
    { mod: { ...chaturProj.modules[4], root: "src-tauri/src" }, warn: {
      kind: "warn",
      body: (
        <span>
          <b>Overlaps with backend.</b>{" "}
          <em>`src-tauri/src` is a parent of `src-tauri/src/engine`, `src-tauri/src/db` and `src-tauri/src/cmds`. Allowed, but jobs will scan the same files multiple times.</em>
        </span>
      )
    }},
  ];
  return (
    <div className="settings-pane">
      <div className="settings-head">
        <div>
          <h2>Modules</h2>
          <div className="sub">2 issues to resolve before queuing a batch.</div>
        </div>
        <div className="settings-head-actions">
          <ScopePill project={chaturProj}/>
        </div>
      </div>
      <div className="settings-body">
        <ModuleListHeader count={5}/>
        <div className="mod-list">
          {rows.map(({ mod, warn }) => <ModRow key={mod.id} mod={mod} warn={warn}/>)}
          <div className="mod-add-row">
            <span className="ic"><Icon name="plus" size={13}/></span>
            <span>Add module…</span>
          </div>
        </div>
      </div>
    </div>
  );
}

/* AI infer — loading */
function PaneInferLoading() {
  return (
    <div className="settings-pane">
      <div className="settings-head">
        <div>
          <h2>Modules · Inferring…</h2>
          <div className="sub">
            Chatur is scanning the project layout. Your current modules stay
            untouched — you'll review a proposal before anything saves.
          </div>
        </div>
        <div className="settings-head-actions">
          <ScopePill project={chaturProj}/>
        </div>
      </div>
      <div className="settings-body">
        <ModuleListHeader count={chaturProj.modules.length}/>
        <div className="infer-loading">
          <div className="spin"/>
          <div className="body">
            <div className="title">Inferring modules with qwen2.5-coder:7b</div>
            <div className="step">Reading package manifests & top-level dirs…</div>
            <div className="steps">
              <div className="s done">Discovering top-level directories <span style={{color:"var(--text-dim)"}}>· 14 found</span></div>
              <div className="s done">Reading package.json, Cargo.toml, tsconfig.json</div>
              <div className="s done">Sampling import graph from 18 entrypoints</div>
              <div className="s cur">Naming and describing module candidates…</div>
              <div className="s">Reconciling with your current modules</div>
            </div>
          </div>
          <button className="btn" style={{alignSelf:"flex-start"}}>
            <Icon name="x" size={12}/>Cancel
          </button>
        </div>
        <div style={{
          marginTop: 12, fontSize: 11, color: "var(--text-dim)",
          fontFamily: "var(--font-mono)",
        }}>
          tokens · 12,480 / ~24,000 · 0:08 elapsed
        </div>
      </div>
    </div>
  );
}

/* AI infer — diff/preview */
function DiffMarker(kind) {
  switch (kind) {
    case "added":   return "+";
    case "removed": return "−";
    case "changed": return "~";
    default:        return "·";
  }
}
function DiffRow({ row, checked = true }) {
  const after = row.after || {};
  const before = row.before || {};
  const isRemoved = row.kind === "removed";
  const data = isRemoved ? before : after;

  // For changed rows, show before→after on whichever fields differ
  const nameCell = (() => {
    if (row.kind === "changed" && before.name !== after.name) {
      return (
        <div className="diff-change">
          <span className="from txt">{before.name}</span>
          <span className="to txt" style={{fontFamily:"var(--font-mono)"}}>{after.name}</span>
        </div>
      );
    }
    return <span style={{fontFamily:"var(--font-mono)"}}>{data.name}</span>;
  })();
  const descCell = (() => {
    if (row.kind === "changed" && before.desc !== after.desc) {
      return (
        <div className="diff-change">
          <span className="from txt">{before.desc}</span>
          <span className="to txt">{after.desc}</span>
        </div>
      );
    }
    return <span>{data.desc}</span>;
  })();
  const pathCell = (() => {
    if (row.kind === "changed" && before.root !== after.root) {
      return (
        <div className="diff-change">
          <span className="from">{before.root}</span>
          <span className="to">{after.root}</span>
        </div>
      );
    }
    return (
      <span>
        {data.root}
        <span className="files">{data.files} files</span>
      </span>
    );
  })();

  return (
    <div className={`diff-row ${row.kind}`}>
      <span className={`check${checked && !isRemoved ? " checked" : ""}`}>
        {checked && !isRemoved && <Icon name="check" size={10}/>}
      </span>
      <span className="marker">{DiffMarker(row.kind)}</span>
      <div className="name">{nameCell}</div>
      <div className="desc">{descCell}</div>
      <div className="path">{pathCell}</div>
      <div className="row-actions">
        {row.kind !== "kept" && (
          <button className="btn-icon" title={isRemoved ? "Keep this module" : "Reject"}>
            <Icon name={isRemoved ? "refresh" : "x"} size={11}/>
          </button>
        )}
      </div>
    </div>
  );
}

function PaneInferDiff() {
  const proposal = M2.inferProposal;
  // Bucket counts
  const counts = proposal.reduce((acc, r) => { acc[r.kind] = (acc[r.kind]||0)+1; return acc; }, {});
  const accepted = (counts.added||0) + (counts.changed||0);

  return (
    <div className="settings-pane">
      <div className="settings-head">
        <div>
          <h2>Modules · Review proposal</h2>
          <div className="sub">
            Chatur proposes these modules based on your repo layout. Cherry-pick
            the rows you want — nothing saves until you click <b style={{color:"var(--text)"}}>Apply</b>.
          </div>
        </div>
        <div className="settings-head-actions">
          <ScopePill project={chaturProj}/>
        </div>
      </div>
      <div className="settings-body">
        <div className="infer-head">
          <span className="ic"><Icon name="sparkles" size={14}/></span>
          <span className="title">Proposal</span>
          <span className="by">by <b>qwen2.5-coder:7b</b> · 14.2s · 23.4k tok</span>
          <div className="legend">
            <span><span className="dot add"/>added</span>
            <span><span className="dot chg"/>changed</span>
            <span><span className="dot rem"/>removed</span>
            <span><span className="dot kep"/>kept</span>
          </div>
        </div>
        <div className="infer-toolbar">
          <span className="count">
            <span className="accepted">{accepted}</span> of {(counts.added||0)+(counts.changed||0)+(counts.removed||0)} changes selected
            <span className="sep">·</span>
            {counts.added||0} new
            <span className="sep">·</span>
            {counts.changed||0} changed
            <span className="sep">·</span>
            {counts.removed||0} removed
            <span className="sep">·</span>
            {counts.kept||0} unchanged
          </span>
          <div className="spacer"/>
          <button className="btn-mini">Reject all</button>
          <button className="btn-mini">Select all</button>
          <button className="btn-accept-all"><Icon name="check" size={11}/> Apply {accepted}</button>
        </div>
        <div className="diff-list">
          {proposal.map((r, i) => (
            <DiffRow key={i} row={r} checked={r.kind !== "removed"}/>
          ))}
        </div>
        <div style={{
          marginTop: 12, fontSize: 11, color: "var(--text-dim)",
          fontFamily: "var(--font-mono)",
        }}>
          tip · removed modules will be detached, not deleted from disk
        </div>
      </div>
    </div>
  );
}

/* ---------- ModulesPane router ---------- */
function ModulesPane({ state }) {
  switch (state) {
    case "empty":         return <PaneEmpty/>;
    case "populated":     return <PanePopulated/>;
    case "adding":        return <PaneAdding/>;
    case "validation":    return <PaneValidation/>;
    case "inferLoading":  return <PaneInferLoading/>;
    case "inferDiff":     return <PaneInferDiff/>;
    default:              return <PanePopulated/>;
  }
}

window.ModulesPane = ModulesPane;
