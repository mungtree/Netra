# AGENTS.md — maintaining the NETRA code-wiki

This file tells an agent (or a human) how to keep the `docs/` code-wiki
accurate and how to add pages to it. Read it fully before editing anything
under `docs/`.

The wiki is a **static HTML site** — no build step, no framework. Open
`docs/index.html` in a browser and everything works from `file://`.

---

## 1. Site structure

```
docs/
├── index.html                  home page
├── AGENTS.md                   this file
├── _TEMPLATE.html              copy-me skeleton for a new page
├── assets/
│   ├── style.css               the ONE shared stylesheet
│   └── nav.js                  the site tree + sidebar injector + filter
├── concepts/                   explanation pages (prose, no code reading)
├── guides/                     build/run guides
└── reference/<crate>/          one page per source file, + an index.html per crate
```

Three layers of content:

- **concepts/** — plain-language explanation. No function-by-function detail.
- **guides/** — how to build and run.
- **reference/** — one HTML page per source file, documenting every public
  type and function.

---

## 2. nav.js is the single source of truth for the site tree

`assets/nav.js` holds a `SITE` array of sections, each with `pages`. The script
injects the sidebar into every page, marks the current page, and renders the
filter box.

**The sidebar is never hand-written in a page.** A page only declares two
`data-*` attributes on its `<body>`; `nav.js` does the rest.

To add a page to the sidebar: add one entry to the right section's `pages`
array in `nav.js`:

```js
{ label: 'newfile.rs', href: 'reference/netra-core/newfile.html', depth: 1 },
```

- `label` — what shows in the sidebar.
- `href` — path **relative to the `docs/` root**.
- `depth: 1` — optional; indents the entry (used for files under a crate).

---

## 3. How to add a page

1. **Copy `_TEMPLATE.html`** to its destination, e.g.
   `docs/reference/netra-core/newfile.html`.
2. **Set the two `data-*` attributes** on `<body>`:
   - `data-root` — relative path from the new file *to the `docs/` root*.
     `.` for `index.html`, `..` for `concepts/` and `guides/`, `../..` for
     `reference/<crate>/`.
   - `data-page` — the file's href relative to `docs/`, e.g.
     `reference/netra-core/newfile.html`. Must match the `nav.js` entry.
3. **Fix the asset/link paths.** In the template, replace every `REL` with the
   same value as `data-root` (the `<link>` to `style.css`, the breadcrumb
   `Home` link, the `<script>` for `nav.js`).
4. **Add the `nav.js` entry** (section 2).
5. **Fill `#content`.** For a reference page, follow the 7-section template
   below.
6. **Verify.** Open the page in a browser: sidebar appears, current page is
   highlighted, every link resolves.

---

## 4. The reference-page template (7 sections)

Every `reference/<crate>/<file>.html` page has the same shape. Reproduce it
exactly so the wiki stays uniform.

1. **Breadcrumb + `<h1>` + `.lead` + `.subtitle`** — crate/file name, a
   one-sentence summary, and the full source path.
2. **`<h2>In plain terms</h2>`** — 2–4 sentences, no jargon. What this file is
   for, explained to a newcomer.
3. **`<h2>Public types</h2>`** — one `.api.type` / `.api.enum` /
   `.api.trait` card per struct/enum/trait: kind, fields/variants, purpose.
4. **`<h2>Functions &amp; methods</h2>`** (or `Methods`, `Trait impl: X`) —
   one `.api.fn` card per public function/method: signature, `file:line` in an
   `.api-loc`, parameters, return, what it does.
5. **Trait impls** — note which trait each type implements (often folded into
   section 4 as a `Trait impl: X` heading).
6. **`.ripple` callout — REQUIRED.** *What calls this, what this calls, and
   what breaks if it changes.* This is the cross-module half of the wiki. Never
   omit it.
7. **`<h2>Related</h2>` + `.pagenav`** — links to neighbouring reference and
   concept pages; previous/next links.

### Filled mini-example

```html
<div class="api fn">
  <span class="api-loc">queue.rs:50</span>
  <span class="api-sig">async fn enqueue(&amp;self, job: Job) -&gt; Result&lt;()&gt;</span>
  <p>Pushes a job to the back of the queue, then signals one waiter.</p>
</div>

<div class="ripple"><span class="ico">&#9889;</span> <strong>Ripple
  effects.</strong> Called by <code>Netra::queue_job</code> and
  <code>BatchExecutor</code>. Its <code>notify_one()</code> wakes a parked
  <code>Scheduler</code>.</div>
```

### CSS classes available (see `assets/style.css`)

| Class | Use |
|-------|-----|
| `.api .type / .enum / .trait / .fn` | a documented item card |
| `.api-sig` | the signature line inside an `.api` card |
| `.api-loc` | the `file:line` tag (floats right) |
| `.ripple` | the cross-module callout (purple) |
| `.note` / `.tip` / `.warn` | blue / green / yellow callouts |
| `.badge struct/enum/trait/fn/crate/async` | inline tags |
| `.diagram` | ASCII diagram box |
| `.grid` + `.tile` | card grid (home / index pages) |
| `.crumb` / `.lead` / `.subtitle` / `.pagenav` / `.muted` | page chrome |

---

## 5. Documentation style rules

- **Plain language first.** Every page opens with an "In plain terms" section a
  non-expert can follow. Jargon comes after, and is defined in
  `concepts/glossary.html`.
- **Document every public item.** Every `pub` type, function, and method gets a
  card. Crate-private items worth understanding (e.g.
  `netra-agent/protocol.rs`) are documented too.
- **Always fill the Ripple-effects callout.** A reference page without it is
  incomplete. State callers, callees, and breakage.
- **Keep `file:line` accurate.** Line numbers drift. When you touch a page,
  re-check them against source.
- **Verify against source — do not trust memory.** Read the actual `.rs` /
  `.svelte` / `.js` file before writing or updating its page.
- **Match the existing tone.** Terse, concrete, second person ("you"). No
  marketing language.

---

## 6. Keeping the wiki in sync with the code

When the codebase changes, update the wiki in the same change:

| Code change | Wiki action |
|-------------|-------------|
| Function signature / behaviour changed | Update that file's reference page (signature, description, `file:line`). |
| New public type / function | Add an `.api` card to the file's page. |
| Cross-module call added/removed | Update the **Ripple effects** on *both* files' pages, and `concepts/cross-module-map.html`. |
| New source file | Copy `_TEMPLATE.html`, write the page, add a `nav.js` entry, link it from the crate `index.html`. |
| Deleted source file | Delete the page, remove its `nav.js` entry and crate-index row, fix inbound links. |
| New crate | Add a `reference/<crate>/` folder with an `index.html`, a new `nav.js` section, and a row in the home page's crate table. |
| Behaviour/architecture change | Re-check the affected `concepts/` page. |

A reusable prompt for each of these is on `docs/prompts.html`.

---

## 7. Pre-publish checklist

Before considering a page done:

- [ ] `data-root` and `data-page` are correct; every `REL` was replaced.
- [ ] The page has a matching entry in `nav.js`.
- [ ] Sidebar renders, current page is highlighted, filter works.
- [ ] Every `<a href>` resolves (no 404 from `file://`).
- [ ] Reference page: all 7 sections present; **Ripple effects filled**.
- [ ] Every signature and `file:line` matches current source.
- [ ] Previous/next `.pagenav` links point somewhere sensible.

---

## 8. Conventions quick-reference

- Reference page filenames flatten subdirectories with `-`:
  `model/job.rs` → `model-job.html`, `repo/job.rs` → `repo-job.html`,
  `traits/bus.rs` → `traits-bus.html`. UI component pages are lowercased:
  `TaskGrid.svelte` → `taskgrid.html`.
- One crate = one `reference/<crate>/` folder with an `index.html` overview.
- Escape HTML in signatures: `&lt; &gt; &amp;`.
- Do not add external CDN dependencies — the site must work offline. (The one
  exception, `concepts/architecture-diagram.html`, loads mermaid from a CDN and
  is a relocated legacy file.)
