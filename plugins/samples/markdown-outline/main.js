// Markdown Outline: a sample Ettin Edit plugin in JavaScript.
//
// Shows the headings of the current tab as a tree in a side panel; click one
// to jump to it. Also adds Insert UUID and Wrap Lines to the Plugins menu.
// Read it alongside the plugin guide (Plugins > Plugin Guide); copy the
// folder to start your own plugin.

'use strict';

const crypto = require('crypto');
const ettin = require('ettin');
const { ui } = ettin;

let visible = false;

/** Headings as {level, title, line}; lines count from 1. */
function headings(text) {
  const lines = text.split('\n');
  const found = [];
  let fence = null;
  lines.forEach((line, i) => {
    const f = line.match(/^\s{0,3}(`{3,}|~{3,})/);
    if (f) {
      if (!fence) fence = f[1][0];
      else if (f[1][0] === fence) fence = null;
      return;
    }
    if (fence) return;
    const atx = line.match(/^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$/);
    if (atx) {
      found.push({ level: atx[1].length, title: atx[2] || '(empty)', line: i + 1 });
      return;
    }
    // "Title" underlined with === or ---.
    const under = line.match(/^\s{0,3}(=+|-+)\s*$/);
    const prev = i > 0 ? lines[i - 1].trim() : '';
    if (under && prev && !/^[-*+>#]/.test(prev)) {
      found.push({ level: under[1][0] === '=' ? 1 : 2, title: prev, line: i });
    }
  });
  return found;
}

/** Nests headings under the ones above them. */
function tree(heads) {
  const root = { level: 0, children: [] };
  const stack = [root];
  for (const h of heads) {
    while (stack[stack.length - 1].level >= h.level) stack.pop();
    const node = ui.node(h.title, { id: String(h.line), detail: `line ${h.line}`, open: true });
    node.level = h.level;
    stack[stack.length - 1].children.push(node);
    stack.push(node);
  }
  return root.children;
}

async function refresh(doc) {
  if (!visible) return;
  doc = doc || (await ettin.activeDocument());
  const heads = headings(await doc.getText());
  const items = heads.length
    ? [ui.tree('outline', tree(heads))]
    : [ui.label('No headings in this tab. Lines that start with # are headings.', { weak: true })];
  await ettin.showPanel('outline', { title: `Outline: ${doc.title}`, items });
}

ettin.onPanel('outline', async (e) => {
  if (e.event === 'close') {
    visible = false;
  } else if (e.event === 'select') {
    const doc = await ettin.activeDocument();
    await doc.goTo(Number(e.value));
  }
});

ettin.command('Show or Hide Markdown Outline', async () => {
  visible = !visible;
  if (visible) await refresh();
  else await ettin.closePanel('outline');
}, { shortcut: 'Ctrl+Alt+O' });

ettin.command('Insert UUID', async () => {
  const doc = await ettin.activeDocument();
  await doc.insert(crypto.randomUUID());
}, { shortcut: 'Ctrl+Alt+I' });

/** Wraps each paragraph at `width` columns. */
function wrap(text, width) {
  return text.split(/\n[ \t]*\n/).map((para) => {
    const indent = para.match(/^[ \t]*/)[0];
    const words = para.trim().split(/\s+/).filter(Boolean);
    const out = [];
    let line = '';
    for (const w of words) {
      if (line && indent.length + line.length + 1 + w.length > width) {
        out.push(indent + line);
        line = w;
      } else {
        line = line ? `${line} ${w}` : w;
      }
    }
    if (line) out.push(indent + line);
    return out.join('\n');
  }).join('\n\n');
}

ettin.command('Wrap Lines…', async () => {
  const doc = await ettin.activeDocument();
  const sels = await doc.getSelections();
  if (!sels.some((s) => s.text)) {
    await ettin.message('Select the lines to wrap first.', 'warning');
    return;
  }
  const answer = await ettin.prompt('Wrap at column:', { value: '80', title: 'Wrap Lines' });
  if (answer === null) return;
  const width = parseInt(answer, 10);
  if (!(width >= 10)) {
    await ettin.message('Enter a column of 10 or more.', 'warning');
    return;
  }
  await doc.replaceSelections(sels.map((s) => wrap(s.text, width)));
});

ettin.on('activated', refresh);
ettin.on('changed', async (doc) => {
  const active = await ettin.activeDocument();
  if (doc.id === active.id) await refresh(doc);
});
