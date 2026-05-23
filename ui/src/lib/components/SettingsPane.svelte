<script>
  import { store, saveSettings } from '$lib/store.svelte.js';

  function clamp(val, min, max) {
    return Math.min(Math.max(Number(val) || min, min), max);
  }

  function stepGlobal(delta) {
    store.settings.globalMax = clamp(store.settings.globalMax + delta, 1, 32);
    if (store.settings.perProjectMax > store.settings.globalMax) {
      store.settings.perProjectMax = store.settings.globalMax;
    }
  }

  function stepPerProject(delta) {
    store.settings.perProjectMax = clamp(
      store.settings.perProjectMax + delta,
      1,
      store.settings.globalMax,
    );
  }

  function onGlobalInput(e) {
    store.settings.globalMax = clamp(e.target.value, 1, 32);
    if (store.settings.perProjectMax > store.settings.globalMax) {
      store.settings.perProjectMax = store.settings.globalMax;
    }
  }

  function onPerProjectInput(e) {
    store.settings.perProjectMax = clamp(e.target.value, 1, store.settings.globalMax);
  }
</script>

<div class="settings-wrap">
  <div class="settings-card">
    <div class="settings-title">Settings</div>

    <!-- Concurrency section -->
    <div class="section">
      <div class="section-label">Concurrency</div>

      <div class="field">
        <div class="field-info">
          <span class="field-name">Max parallel agents</span>
          <span class="field-desc">Total pi processes running at once across all projects</span>
        </div>
        <div class="stepper">
          <button class="step-btn" onclick={() => stepGlobal(-1)} disabled={store.settings.globalMax <= 1}>−</button>
          <input
            type="number"
            class="step-input"
            min="1"
            max="32"
            value={store.settings.globalMax}
            oninput={onGlobalInput}
          />
          <button class="step-btn" onclick={() => stepGlobal(1)} disabled={store.settings.globalMax >= 32}>+</button>
        </div>
      </div>

      <div class="field">
        <div class="field-info">
          <span class="field-name">Max per project</span>
          <span class="field-desc">Cap on concurrent agents for any single project</span>
        </div>
        <div class="stepper">
          <button class="step-btn" onclick={() => stepPerProject(-1)} disabled={store.settings.perProjectMax <= 1}>−</button>
          <input
            type="number"
            class="step-input"
            min="1"
            max={store.settings.globalMax}
            value={store.settings.perProjectMax}
            oninput={onPerProjectInput}
          />
          <button class="step-btn" onclick={() => stepPerProject(1)} disabled={store.settings.perProjectMax >= store.settings.globalMax}>+</button>
        </div>
      </div>
    </div>

    <!-- Runtime section -->
    <div class="section">
      <div class="section-label">Runtime</div>

      <div class="field">
        <div class="field-info">
          <span class="field-name">Pi binary</span>
          <span class="field-desc">Path to the pi executable, or just <code>pi</code> if on PATH</span>
        </div>
        <input
          type="text"
          class="text-input"
          bind:value={store.settings.piBinary}
          placeholder="pi"
          spellcheck="false"
        />
      </div>
    </div>

    <!-- Agent tools + system prompt -->
    <div class="section">
      <div class="section-label">Agent</div>

      <div class="field stacked">
        <div class="field-info">
          <span class="field-name">Tool access</span>
          <span class="field-desc">
            Controls which built-in <code>pi</code> tools the agent may call.
            <code>read</code> only is safest;
            <code>read + bash</code> lets the agent run <code>ls</code>, <code>grep</code>, <code>find</code> (and any other shell command);
            <code>full</code> also allows <code>edit</code> and <code>write</code>.
          </span>
        </div>
        <div class="radio-group">
          <label class="radio">
            <input
              type="radio"
              name="tools-mode"
              value="read"
              checked={store.settings.toolsMode === 'read'}
              onchange={() => (store.settings.toolsMode = 'read')}
            />
            <span>Read only</span>
          </label>
          <label class="radio">
            <input
              type="radio"
              name="tools-mode"
              value="read_bash"
              checked={store.settings.toolsMode === 'read_bash'}
              onchange={() => (store.settings.toolsMode = 'read_bash')}
            />
            <span>Read + bash</span>
          </label>
          <label class="radio">
            <input
              type="radio"
              name="tools-mode"
              value="full"
              checked={store.settings.toolsMode === 'full'}
              onchange={() => (store.settings.toolsMode = 'full')}
            />
            <span>Full (read, bash, edit, write)</span>
          </label>
        </div>
      </div>

      <div class="field stacked">
        <div class="field-info">
          <span class="field-name">Appended system prompt</span>
          <span class="field-desc">
            Text appended to pi's default system prompt for every job (passed via
            <code>--append-system-prompt</code>). Useful for project-wide guardrails — e.g.,
            telling the agent which bash commands it may or may not run. Leave blank to use pi's default prompt only.
          </span>
        </div>
        <textarea
          class="text-area"
          rows="6"
          spellcheck="false"
          placeholder="e.g. You may only use ls, grep, find, cat for inspection. Do not run build/test/install commands."
          bind:value={store.settings.systemPromptAppend}
        ></textarea>
      </div>
    </div>

    <!-- Save row -->
    <div class="save-row">
      <button class="btn" onclick={saveSettings}>Save settings</button>
      {#if store.settingsSaved}
        <span class="saved-note">Saved — restart the app for changes to take effect</span>
      {/if}
    </div>
  </div>
</div>

<style>
  .settings-wrap {
    flex: 1;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding: 40px 24px;
    overflow-y: auto;
    background: var(--bg);
  }

  .settings-card {
    width: 100%;
    max-width: 560px;
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .settings-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text);
    letter-spacing: -0.01em;
    margin-bottom: 28px;
  }

  .section {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-panel);
    overflow: hidden;
    margin-bottom: 12px;
  }

  .section-label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
    padding: 10px 16px 8px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-elev);
  }

  .field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .field:last-child {
    border-bottom: none;
  }

  .field-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .field-name {
    font-size: 13px;
    color: var(--text);
    font-weight: 500;
  }

  .field-desc {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  .field-desc code {
    font-family: var(--font-mono);
    font-size: 10px;
    background: var(--bg-active);
    color: var(--accent-soft);
    padding: 1px 4px;
    border-radius: 3px;
  }

  /* Stepper */
  .stepper {
    display: flex;
    align-items: center;
    gap: 0;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .step-btn {
    width: 28px;
    height: 28px;
    background: var(--bg-elev);
    border: none;
    color: var(--text-muted);
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.1s, color 0.1s;
    flex-shrink: 0;
  }

  .step-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text);
  }

  .step-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .step-input {
    width: 48px;
    height: 28px;
    background: var(--bg-panel);
    border: none;
    border-left: 1px solid var(--border);
    border-right: 1px solid var(--border);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 13px;
    text-align: center;
    outline: none;
    -moz-appearance: textfield;
  }

  .step-input::-webkit-outer-spin-button,
  .step-input::-webkit-inner-spin-button {
    -webkit-appearance: none;
  }

  /* Text input */
  .text-input {
    width: 200px;
    height: 30px;
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 0 10px;
    outline: none;
    flex-shrink: 0;
    transition: border-color 0.15s;
  }

  .text-input:focus {
    border-color: var(--accent-border);
  }

  /* Stacked field: label above control, full width */
  .field.stacked {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
  }

  /* Radio group */
  .radio-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .radio {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
  }

  .radio input[type='radio'] {
    accent-color: var(--accent, #6aa9ff);
    margin: 0;
  }

  /* Textarea */
  .text-area {
    width: 100%;
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.4;
    padding: 8px 10px;
    outline: none;
    resize: vertical;
    transition: border-color 0.15s;
    box-sizing: border-box;
  }

  .text-area:focus {
    border-color: var(--accent-border);
  }

  /* Save row */
  .save-row {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-top: 4px;
  }

  .saved-note {
    font-size: 11px;
    color: var(--text-muted);
  }
</style>
