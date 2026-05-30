// modules-shell.jsx
// Reusable chrome wrapper for each mockup artboard.
// Mirrors Titlebar / ActivityBar / Sidebar / StatusBar from app.jsx so every
// surface lives inside the real Chatur shell.

const D = window.CHATUR_DATA;
const M = window.MODULES_DATA;

/* ---------- Titlebar ---------- */
function ShellTitlebar() {
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

/* ---------- Activity bar ----------
   `active` controls which activity icon is highlighted. */
function ShellActivityBar({ active = "projects" }) {
  const items = [
    { id: "projects", icon: "folder" },
    { id: "modules",  icon: "layers" },
    { id: "library",  icon: "library" },
    { id: "prompts",  icon: "prompt" },
    { id: "chroma",   icon: "db"     },
    { id: "history",  icon: "clock"  },
    { id: "activity", icon: "activity" },
  ];
  return (
    <div className="activitybar">
      {items.map(i => (
        <a key={i.id} className={`ab-btn${i.id === active ? " active" : ""}`}>
          <Icon name={i.icon}/>
        </a>
      ))}
      <div className="ab-spacer"/>
      <a className={`ab-btn${active === "settings" ? " active" : ""}`}><Icon name="settings"/></a>
    </div>
  );
}

/* ---------- Sidebar ----------
   activeId controls which project row is highlighted. The sidebar header
   text can also be replaced for the "Projects overview" page where it
   shows folders/groupings. */
function ShellSidebar({ activeId = "chatur" }) {
  return (
    <div className="sidebar">
      <div className="sb-header">
        <div className="sb-title">Projects</div>
        <button className="sb-add" title="Add project"><Icon name="plus" size={14}/></button>
      </div>
      <div className="sb-search">
        <Icon name="search" size={13}/>
        <input placeholder="Search projects…" readOnly/>
        <span className="kbd">⌘P</span>
      </div>
      <div className="proj-list">
        {M.projects.map(p => (
          <div key={p.id} className={`proj-item${p.id === activeId ? " active" : ""}`}>
            <span className={`proj-status ${p.status}`}/>
            <div className="proj-info">
              <div className="proj-name">{p.name}</div>
              <div className="proj-path">{p.path}</div>
            </div>
            <div className="proj-count">{p.modules.length > 1 ? `${p.modules.length} mods` : ""}</div>
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

/* ---------- Status bar ---------- */
function ShellStatusBar({ queued = 3, running = 1, done = 14 }) {
  return (
    <div className="statusbar">
      <span className="sb-item"><span className="ic"><Icon name="branch" size={11}/></span>main</span>
      <span className="sb-item"><span className="ic"><Icon name="cpu" size={11}/></span>qwen2.5-coder:7b · 4.2GB</span>
      <span className="sb-item"><span className="ic"><Icon name="activity" size={11}/></span>GPU 71% · 12.4 tok/s</span>
      <span className="statusbar-spacer"/>
      <span className="sb-item accent">● {running} running</span>
      <span className="sb-item">{queued} queued</span>
      <span className="sb-item">{done} done today</span>
      <span className="sb-item">tauri · 0.4.2</span>
    </div>
  );
}

/* ---------- Window frame ----------
   Wraps the four chrome pieces + a slot for main + optional right rail. */
function ChaturWindow({
  activeActivity = "projects",
  activeProject = "chatur",
  children,
  rightRail = null,
  statusOverrides = {},
}) {
  return (
    <div className="win-frame" data-screen-label="">
      <ShellTitlebar/>
      <div className="body">
        <ShellActivityBar active={activeActivity}/>
        <ShellSidebar activeId={activeProject}/>
        <div className="main">{children}</div>
        {rightRail}
      </div>
      <ShellStatusBar {...statusOverrides}/>
    </div>
  );
}

/* ---------- Settings sub-nav ----------
   Lives inside the main pane for any Settings-* artboard. */
function SettingsNav({ active = "modules" }) {
  const groups = [
    { label: "PROJECT", items: [
      { id: "general",  label: "General",       icon: "settings" },
      { id: "modules",  label: "Modules",       icon: "layers", count: 5 },
      { id: "prompts",  label: "Prompts",       icon: "prompt", count: 47 },
      { id: "ignore",   label: "Ignore paths",  icon: "file" },
    ]},
    { label: "ENGINE", items: [
      { id: "models",   label: "Models",        icon: "cpu" },
      { id: "tools",    label: "Tool policy",   icon: "shield" },
      { id: "limits",   label: "Limits & cost", icon: "gauge" },
    ]},
  ];
  return (
    <div className="settings-nav">
      {groups.map(g => (
        <React.Fragment key={g.label}>
          <div className="group">{g.label}</div>
          {g.items.map(it => (
            <div key={it.id} className={`item${it.id === active ? " active" : ""}`}>
              <span className="ic"><Icon name={it.icon} size={14}/></span>
              <span>{it.label}</span>
              {it.count != null && <span className="count">{it.count}</span>}
            </div>
          ))}
        </React.Fragment>
      ))}
    </div>
  );
}

window.ChaturWindow = ChaturWindow;
window.SettingsNav = SettingsNav;
