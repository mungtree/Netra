// Chatur — Prompt editor page
const { useState, useMemo, useRef, useEffect } = React;
const PD = window.CHATUR_PROMPTS;

const FORMAT = 'chatur.batch/v1';

/* ---------- helpers ---------- */
function cryptoId() {
  return Math.random().toString(36).slice(2, 10);
}

function serializeBatch(preset) {
  // Mirror the spec's serializer, then optionally bake the stop condition
  // into the prompts so the runner sees it.
  const cond = PD.stopConditions.find(s => s.id === preset.stopConditionId) || PD.stopConditions[0];
  const stopText = cond.id === 'custom' ? (preset.customStopText || '') : cond.text;
  const baked = (preset.prompts || []).map((p, i) => {
    if (!stopText) return p;
    const apply = preset.appendToAll || i === preset.prompts.length - 1;
    if (!apply) return p;
    return `${p}\n\n— Stop condition —\n${stopText}`;
  });
  return JSON.stringify({
    format: FORMAT,
    name: preset.title || 'Untitled batch',
    strategy: preset.strategy,
    prompts: baked,
    output_schema: preset.output_schema ?? null,
  }, null, 2);
}

function highlightJSON(json) {
  const esc = json
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
  return esc.replace(
    /("(?:\\.|[^"\\])*"\s*:?)|\b(true|false|null)\b|\b(-?\d+(?:\.\d+)?)\b/g,
    (m, str, bool, num) => {
      if (str) {
        const isKey = /:$/.test(str);
        const body = isKey ? str.slice(0, -1) : str;
        return `<span class="${isKey ? 'k' : 's'}">${body}</span>${isKey ? '<span class="p">:</span>' : ''}`;
      }
      if (bool) return `<span class="b">${bool}</span>`;
      if (num) return `<span class="n">${num}</span>`;
      return m;
    },
  );
}

/* ---------- Custom select with popover ---------- */
function PESelect({ options, value, onChange, renderItem, renderValue }) {
  const [open, setOpen] = useState(false);
  const ref = useRef(null);
  useEffect(() => {
    if (!open) return;
    const onDown = e => { if (ref.current && !ref.current.contains(e.target)) setOpen(false); };
    document.addEventListener('mousedown', onDown);
    return () => document.removeEventListener('mousedown', onDown);
  }, [open]);
  const selected = options.find(o => o.id === value);
  return (
    <div ref={ref} className={`pe-select${open ? ' open' : ''}`} onClick={() => setOpen(o => !o)}>
      {renderValue ? renderValue(selected) : (
        <div className="left">
          <span className="name">{selected?.name}</span>
          {selected?.desc && <span className="desc">{selected.desc}</span>}
        </div>
      )}
      <span className="arrow"><Icon name="chevronDown" size={14}/></span>
      {open && (
        <div className="pe-popover" onClick={e => e.stopPropagation()}>
          {options.map(opt => (
            <div
              key={opt.id}
              className={`pe-popover-item${opt.id === value ? ' selected' : ''}${opt.muted ? ' muted' : ''}`}
              onClick={() => { onChange(opt.id); setOpen(false); }}
            >
              <div style={{flex: 1, minWidth: 0}}>
                <div className="name">{opt.name}{opt.code && <code>{opt.code}</code>}</div>
                {opt.desc && <div className="desc">{opt.desc}</div>}
              </div>
              {opt.id === value && <span className="tick"><Icon name="check" size={13}/></span>}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

/* ---------- Preset rail ---------- */
function PresetRail({ presets, activeId, onPick, onNew, onImport }) {
  const [q, setQ] = useState('');
  const filtered = presets.filter(p => p.title.toLowerCase().includes(q.toLowerCase()));
  const builtins = filtered.filter(p => p.builtin);
  const customs  = filtered.filter(p => !p.builtin);
  return (
    <div className="pe-rail">
      <div className="pe-rail-head">
        <div className="ttl">Batches</div>
        <div className="pe-rail-actions">
          <button className="btn-icon" title="Import JSON" onClick={onImport}><Icon name="upload" size={14}/></button>
          <button className="btn-icon" title="New batch" onClick={onNew}><Icon name="plus" size={14}/></button>
        </div>
      </div>
      <div className="pe-rail-search">
        <Icon name="search" size={13}/>
        <input placeholder="Search batches…" value={q} onChange={e => setQ(e.target.value)}/>
      </div>
      {builtins.length > 0 && <div className="pe-rail-section">Built-in</div>}
      <div className="pe-rail-list">
        {builtins.map(p => (
          <PresetItem key={p.id} p={p} active={p.id === activeId} onClick={() => onPick(p.id)}/>
        ))}
        {customs.length > 0 && <div className="pe-rail-section" style={{marginTop:8}}>Custom</div>}
        {customs.map(p => (
          <PresetItem key={p.id} p={p} active={p.id === activeId} onClick={() => onPick(p.id)}/>
        ))}
        {filtered.length === 0 && (
          <div style={{padding:'18px 12px', fontSize:11.5, color:'var(--text-dim)', textAlign:'center'}}>
            No batches match "{q}"
          </div>
        )}
      </div>
    </div>
  );
}

function PresetItem({ p, active, onClick }) {
  return (
    <div className={`pe-preset${active ? ' active' : ''}`} onClick={onClick}>
      <span className="pi-icon"><Icon name={p.icon || 'bookmark'} size={13}/></span>
      <div className="pi-body">
        <div className="pi-title">
          {p.title}
          {!p.builtin && <span className="pi-tag">CUSTOM</span>}
        </div>
        <div className="pi-meta">{p.prompts.length} prompts · {p.strategy}</div>
      </div>
    </div>
  );
}

/* ---------- Editor head ---------- */
function EditorHead({ preset, dirty, update }) {
  const cond = PD.stopConditions.find(s => s.id === preset.stopConditionId) || PD.stopConditions[0];
  const stopText = cond.id === 'custom' ? preset.customStopText : cond.text;
  return (
    <div className="pe-editor-head">
      <div className="pe-title-row">
        <input
          className="pe-title-input"
          value={preset.title}
          onChange={e => update({ title: e.target.value })}
          placeholder="Untitled batch"
          spellCheck={false}
        />
        <span className={`pe-badge${dirty ? ' dirty' : ''}`}>{dirty ? '● UNSAVED' : 'SAVED'}</span>
        <span className="pe-badge">{FORMAT}</span>
      </div>
      <div className="pe-controls">
        <div className="pe-field">
          <label className="pe-field-label">
            Strategy
            <span className="hint">— how prompts compose into a run</span>
          </label>
          <PESelect
            options={PD.strategies.map(s => ({ id: s.id, name: s.name, code: s.id, desc: s.desc }))}
            value={preset.strategy}
            onChange={v => update({ strategy: v })}
          />
        </div>
        <div className="pe-field">
          <label className="pe-field-label">
            Stop condition
            <span className="hint">— appended to prompts at run time</span>
          </label>
          <PESelect
            options={PD.stopConditions.map(s => ({ id: s.id, name: s.name, desc: s.desc, muted: s.id === 'default' }))}
            value={preset.stopConditionId}
            onChange={v => update({ stopConditionId: v })}
          />
        </div>
      </div>

      <div className="pe-stop-extra">
        {cond.id === 'custom' ? (
          <>
            <div style={{fontFamily:'var(--font-mono)', fontSize:10, letterSpacing:'.14em', textTransform:'uppercase', color:'var(--text-muted)'}}>Custom halt rule</div>
            <textarea
              className="pe-custom-stop"
              value={preset.customStopText}
              onChange={e => update({ customStopText: e.target.value })}
              placeholder="e.g. When you have nothing more to add, reply with FINISHED on a single line."
              spellCheck={false}
            />
          </>
        ) : cond.id === 'default' ? (
          <div className="pe-stop-preview">
            <span className="placeholder">No text is appended — the runner uses {preset.strategy}'s built-in halt rule.</span>
          </div>
        ) : (
          <div className="pe-stop-preview">{cond.text}</div>
        )}

        <div className="pe-stop-options">
          <label className="pe-toggle">
            <input
              type="checkbox"
              checked={preset.appendToAll}
              onChange={e => update({ appendToAll: e.target.checked })}
              disabled={cond.id === 'default'}
            />
            <span className="sw"/>
            <span>Append to every prompt</span>
          </label>
          <span style={{color:'var(--text-dim)', fontFamily:'var(--font-mono)', fontSize:10.5}}>
            {cond.id === 'default'
              ? 'disabled — strategy default'
              : preset.appendToAll
                ? `${preset.prompts.length} of ${preset.prompts.length} prompts`
                : `1 of ${preset.prompts.length} prompts (last only)`}
          </span>
        </div>
      </div>
    </div>
  );
}

/* ---------- Prompts list ---------- */
function PromptsList({ preset, update }) {
  const cond = PD.stopConditions.find(s => s.id === preset.stopConditionId) || PD.stopConditions[0];
  const hasStop = cond.id !== 'default' && (cond.id !== 'custom' || preset.customStopText.trim());

  const setPrompt = (i, v) => {
    const next = [...preset.prompts];
    next[i] = v;
    update({ prompts: next });
  };
  const removePrompt = i => {
    if (preset.prompts.length <= 1) return;
    update({ prompts: preset.prompts.filter((_, idx) => idx !== i) });
  };
  const duplicatePrompt = i => {
    const next = [...preset.prompts];
    next.splice(i + 1, 0, preset.prompts[i]);
    update({ prompts: next });
  };
  const movePrompt = (i, dir) => {
    const j = i + dir;
    if (j < 0 || j >= preset.prompts.length) return;
    const next = [...preset.prompts];
    [next[i], next[j]] = [next[j], next[i]];
    update({ prompts: next });
  };
  const addPrompt = () => update({ prompts: [...preset.prompts, ''] });

  return (
    <>
      <div className="pe-section-head">
        <h3>Prompts</h3>
        <span className="count">{preset.prompts.length} step{preset.prompts.length === 1 ? '' : 's'}</span>
        <div className="spacer"/>
        <span className="hint">{preset.strategy === 'reviewer' ? 'first is draft · rest critique' : preset.strategy === 'schema_merge' ? 'each fills a slice of schema' : preset.strategy === 'structured_reviewer' ? 'each must emit schema-valid JSON' : 'outputs concatenated in order'}</span>
      </div>
      <div className="pe-prompts">
        {preset.prompts.map((text, i) => {
          const stopApplies = hasStop && (preset.appendToAll || i === preset.prompts.length - 1);
          return (
            <div key={i} className="pe-prompt">
              <div className="pe-prompt-gutter">
                <span className="pe-prompt-num">{String(i + 1).padStart(2, '0')}</span>
                <button className="pe-prompt-handle" title="Move up" onClick={() => movePrompt(i, -1)} disabled={i === 0}>
                  <Icon name="drag" size={12}/>
                </button>
              </div>
              <div className="pe-prompt-main">
                <textarea
                  className="pe-prompt-text"
                  value={text}
                  onChange={e => setPrompt(i, e.target.value)}
                  placeholder={`Prompt ${i + 1}…`}
                  spellCheck={false}
                />
                <div className="pe-prompt-foot">
                  <span>{text.length} chars · ~{Math.ceil(text.length / 4)} tokens</span>
                  {stopApplies && (
                    <span className="stop-tag" title="Stop condition will be appended to this prompt">
                      <Icon name="stop" size={9}/>
                      stop appended
                    </span>
                  )}
                  <div className="spacer"/>
                  <button title="Move up" onClick={() => movePrompt(i, -1)} disabled={i === 0}>↑</button>
                  <button title="Move down" onClick={() => movePrompt(i, 1)} disabled={i === preset.prompts.length - 1}>↓</button>
                  <button title="Duplicate" onClick={() => duplicatePrompt(i)}><Icon name="copy" size={11}/></button>
                  <button className="danger" title="Delete" onClick={() => removePrompt(i)} disabled={preset.prompts.length === 1}>
                    <Icon name="trash" size={11}/>
                  </button>
                </div>
              </div>
            </div>
          );
        })}
      </div>
      <button className="pe-add-prompt" onClick={addPrompt}>
        <Icon name="plus" size={13}/>
        Add prompt
      </button>
    </>
  );
}

/* ---------- Schema panel ---------- */
function SchemaPanel({ preset, update }) {
  const needsSchema = preset.strategy === 'structured_reviewer' || preset.strategy === 'schema_merge';
  const [open, setOpen] = useState(!!preset.output_schema || needsSchema);
  const [text, setText] = useState(preset.output_schema ? JSON.stringify(preset.output_schema, null, 2) : '');
  const [err, setErr] = useState('');

  useEffect(() => {
    setText(preset.output_schema ? JSON.stringify(preset.output_schema, null, 2) : '');
    setErr('');
  }, [preset.id]);

  const onText = v => {
    setText(v);
    if (!v.trim()) {
      setErr('');
      update({ output_schema: null });
      return;
    }
    try {
      const parsed = JSON.parse(v);
      setErr('');
      update({ output_schema: parsed });
    } catch (e) {
      setErr(e.message);
    }
  };

  return (
    <div className={`pe-schema${open ? ' open' : ''}`}>
      <div className="pe-schema-head" onClick={() => setOpen(o => !o)}>
        <Icon name="layers" size={13} style={{color: needsSchema ? 'var(--accent)' : 'var(--text-muted)'}}/>
        <span className="ttl">Output schema</span>
        <span className="meta">
          {preset.output_schema ? 'set' : needsSchema ? 'required by strategy · not set' : 'optional · null'}
        </span>
        <div className="spacer"/>
        <span className="chev"><Icon name="chevronDown" size={14}/></span>
      </div>
      {open && (
        <div className="pe-schema-body">
          <textarea
            className={`pe-schema-textarea${err ? ' err' : ''}`}
            value={text}
            onChange={e => onText(e.target.value)}
            placeholder='{\n  "type": "array",\n  "items": { ... }\n}'
            spellCheck={false}
          />
          {err && <div className="pe-schema-err">JSON error: {err}</div>}
        </div>
      )}
    </div>
  );
}

/* ---------- Preview rail ---------- */
function PreviewRail({ preset, onToast }) {
  const [tab, setTab] = useState('json');
  const json = useMemo(() => serializeBatch(preset), [preset]);

  const cond = PD.stopConditions.find(s => s.id === preset.stopConditionId) || PD.stopConditions[0];
  const stopText = cond.id === 'custom' ? (preset.customStopText || '') : cond.text;
  const hasStop = cond.id !== 'default' && stopText.trim();

  const copy = () => {
    navigator.clipboard.writeText(json).then(
      () => onToast('Copied JSON to clipboard'),
      () => onToast('Copy failed', true),
    );
  };
  const download = () => {
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${(preset.title || 'batch').toLowerCase().replace(/[^a-z0-9]+/g, '-')}.json`;
    a.click();
    URL.revokeObjectURL(url);
    onToast('Downloaded ' + a.download);
  };

  return (
    <div className="pe-preview">
      <div className="pe-preview-head">
        <span className="ttl">Output</span>
        <div className="spacer"/>
        <div className="pe-preview-tabs">
          <button className={tab === 'json' ? 'active' : ''} onClick={() => setTab('json')}>JSON</button>
          <button className={tab === 'resolved' ? 'active' : ''} onClick={() => setTab('resolved')}>RESOLVED</button>
        </div>
      </div>
      <div className="pe-preview-actions">
        <button className="btn" onClick={copy}><Icon name="copy" size={12}/>Copy</button>
        <button className="btn btn-primary" onClick={download}><Icon name="download" size={12}/>Export</button>
      </div>
      <div className="pe-preview-body">
        {tab === 'json' ? (
          <pre className="pe-json" dangerouslySetInnerHTML={{__html: highlightJSON(json)}}/>
        ) : (
          <div className="pe-resolved">
            {preset.prompts.map((p, i) => {
              const apply = hasStop && (preset.appendToAll || i === preset.prompts.length - 1);
              return (
                <div key={i} className="pe-resolved-item">
                  <div className="hdr">
                    <span className="num">{String(i + 1).padStart(2, '0')}</span>
                    <span>·</span>
                    <span>{p.length + (apply ? stopText.length + 22 : 0)} chars</span>
                    {apply && <><span>·</span><span style={{color:'var(--accent)'}}>+ stop</span></>}
                  </div>
                  <div className="body">
                    {p || <span style={{color:'var(--text-dim)', fontStyle:'italic'}}>(empty)</span>}
                    {apply && (
                      <div className="appended">
                        <span className="lbl">— Stop condition —</span>
                        {stopText}
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

/* ---------- Toolbar above prompts ---------- */
function EditorToolbar({ preset, onDelete, onDuplicate }) {
  const totalChars = preset.prompts.reduce((s, p) => s + p.length, 0);
  return (
    <div className="pe-toolbar">
      <div className="meta">
        <span><span className="k">prompts</span><span className="v">{preset.prompts.length}</span></span>
        <span><span className="k">chars</span><span className="v">{totalChars.toLocaleString()}</span></span>
        <span><span className="k">~tokens</span><span className="v">{Math.ceil(totalChars / 4).toLocaleString()}</span></span>
        <span><span className="k">strategy</span><span className="v">{preset.strategy}</span></span>
      </div>
      <div className="spacer"/>
      <button className="btn" onClick={onDuplicate}><Icon name="copy" size={12}/>Duplicate</button>
      <button className="btn" onClick={onDelete} disabled={preset.builtin} title={preset.builtin ? "Built-in batches can't be deleted" : "Delete"}>
        <Icon name="trash" size={12}/>Delete
      </button>
      <button className="btn btn-primary"><Icon name="play" size={12}/>Run batch</button>
    </div>
  );
}

/* ---------- App ---------- */
function App() {
  const [presets, setPresets] = useState(PD.presets);
  const [activeId, setActiveId] = useState(presets[0].id);
  const [savedSnapshot, setSavedSnapshot] = useState(() => JSON.stringify(presets[0]));
  const [toast, setToast] = useState(null);
  const fileInputRef = useRef(null);

  const active = presets.find(p => p.id === activeId) || presets[0];
  const dirty = JSON.stringify(active) !== savedSnapshot;

  const showToast = (msg, err) => {
    setToast({ msg, err: !!err });
    setTimeout(() => setToast(null), 2200);
  };

  const update = patch => {
    setPresets(ps => ps.map(p => p.id === activeId ? { ...p, ...patch } : p));
  };

  const pick = id => {
    setActiveId(id);
    const next = presets.find(p => p.id === id);
    setSavedSnapshot(JSON.stringify(next));
  };

  const newBatch = () => {
    const id = `custom-${cryptoId()}`;
    const fresh = {
      id, icon: 'bookmark', title: 'Untitled batch', desc: '1 prompt · concat',
      strategy: 'concat', stopConditionId: 'default', customStopText: '',
      appendToAll: false, output_schema: null, builtin: false,
      prompts: ['Write your first prompt here…'],
    };
    setPresets(ps => [...ps, fresh]);
    setActiveId(id);
    setSavedSnapshot(JSON.stringify(fresh));
    showToast('New batch created');
  };

  const duplicate = () => {
    const id = `custom-${cryptoId()}`;
    const copy = { ...active, id, title: `${active.title} (copy)`, builtin: false };
    setPresets(ps => [...ps, copy]);
    setActiveId(id);
    setSavedSnapshot(JSON.stringify(copy));
    showToast('Duplicated');
  };

  const remove = () => {
    if (active.builtin) return;
    const idx = presets.findIndex(p => p.id === activeId);
    const next = presets.filter(p => p.id !== activeId);
    setPresets(next);
    const newActive = next[Math.max(0, idx - 1)];
    setActiveId(newActive.id);
    setSavedSnapshot(JSON.stringify(newActive));
    showToast('Deleted batch');
  };

  const importJSON = () => fileInputRef.current?.click();
  const onImportFile = e => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      try {
        const data = JSON.parse(reader.result);
        if (data.format !== FORMAT) throw new Error(`Unexpected format: ${data.format}`);
        const id = `custom-${cryptoId()}`;
        const fresh = {
          id, icon: 'bookmark', title: data.name || 'Imported batch',
          desc: `${(data.prompts || []).length} prompts · ${data.strategy}`,
          strategy: data.strategy, stopConditionId: 'default', customStopText: '',
          appendToAll: false, output_schema: data.output_schema ?? null, builtin: false,
          prompts: data.prompts || [''],
        };
        setPresets(ps => [...ps, fresh]);
        setActiveId(id);
        setSavedSnapshot(JSON.stringify(fresh));
        showToast(`Imported "${fresh.title}"`);
      } catch (err) {
        showToast(`Import failed: ${err.message}`, true);
      }
    };
    reader.readAsText(file);
    e.target.value = '';
  };

  const save = () => {
    setSavedSnapshot(JSON.stringify(active));
    showToast('Saved');
  };

  // Cmd-S to save
  useEffect(() => {
    const onKey = e => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        save();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  return (
    <>
      <Titlebar/>
      <div className="body">
        <ActivityBar/>
        <div className="pe-layout">
          <PresetRail
            presets={presets}
            activeId={activeId}
            onPick={pick}
            onNew={newBatch}
            onImport={importJSON}
          />
          <div className="pe-editor">
            <EditorHead preset={active} dirty={dirty} update={update}/>
            <EditorToolbar preset={active} onDelete={remove} onDuplicate={duplicate}/>
            <div className="pe-body">
              <PromptsList preset={active} update={update}/>
              <SchemaPanel preset={active} update={update}/>
            </div>
          </div>
          <PreviewRail preset={active} onToast={showToast}/>
        </div>
      </div>
      <StatusBar dirty={dirty}/>
      <input ref={fileInputRef} type="file" accept="application/json,.json" style={{display:'none'}} onChange={onImportFile}/>
      {toast && (
        <div className={`pe-toast${toast.err ? ' err' : ''}`}>
          <Icon name={toast.err ? 'x' : 'check'} size={13}/>
          {toast.msg}
        </div>
      )}
    </>
  );
}

/* ---------- Shared chrome (local copies to keep this file self-contained) ---------- */
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
        <span>⌘S to save</span>
      </div>
    </div>
  );
}

function ActivityBar() {
  const items = [
    { id: "projects", icon: "folder",  href: "Chatur.html" },
    { id: "library",  icon: "library", href: "#" },
    { id: "prompts",  icon: "prompt",  href: "Prompts.html", active: true },
    { id: "history",  icon: "clock",   href: "Review.html" },
    { id: "activity", icon: "activity", href: "#" },
  ];
  return (
    <div className="activitybar">
      {items.map(i => (
        <a key={i.id} href={i.href} className={`ab-btn${i.active ? " active" : ""}`} title={i.id}>
          <Icon name={i.icon}/>
        </a>
      ))}
      <div className="ab-spacer"/>
      <button className="ab-btn" title="Settings"><Icon name="settings"/></button>
    </div>
  );
}

function StatusBar({ dirty }) {
  return (
    <div className="statusbar">
      <span className="sb-item"><span className="ic"><Icon name="prompt" size={11}/></span>prompt editor</span>
      <span className="sb-item"><span className="ic"><Icon name="cpu" size={11}/></span>qwen2.5-coder:7b · 4.2GB</span>
      <span className="sb-item"><span className="ic"><Icon name="file" size={11}/></span>{FORMAT}</span>
      <span className="statusbar-spacer"/>
      {dirty && <span className="sb-item accent">● unsaved</span>}
      <span className="sb-item">⌘S save</span>
      <span className="sb-item">tauri · 0.4.2</span>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("app")).render(<App/>);
