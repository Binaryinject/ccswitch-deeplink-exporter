import { invoke } from "@tauri-apps/api/core";

interface DeeplinkItem {
  item_type: string;
  name: string;
  app: string;
  deeplink: string;
  endpoint: string;
  model: string;
  is_current: boolean;
}

let allItems: DeeplinkItem[] = [];

// DOM
const cardList = document.getElementById("card-list")!;
const statsEl = document.getElementById("stats")!;
const dbPathEl = document.getElementById("db-path")!;
const toastEl = document.getElementById("toast")!;
const btnCopyAll = document.getElementById("btn-copy-all")!;
const btnExport = document.getElementById("btn-export")!;
const btnRefresh = document.getElementById("btn-refresh")!;
const filterBtns = document.querySelectorAll<HTMLButtonElement>(".filter-btn");

function showToast(msg: string) {
  toastEl.textContent = msg;
  toastEl.classList.remove("hidden");
  setTimeout(() => toastEl.classList.add("hidden"), 2000);
}

async function copyToClipboard(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    showToast("已复制!");
  } catch {
    // fallback
    const ta = document.createElement("textarea");
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand("copy");
    document.body.removeChild(ta);
    showToast("已复制!");
  }
}

function escapeHtml(s: string): string {
  const div = document.createElement("div");
  div.textContent = s;
  return div.innerHTML;
}

function badgeClass(app: string): string {
  switch (app) {
    case "claude": return "badge-claude";
    case "codex": return "badge-codex";
    case "gemini": return "badge-gemini";
    case "skill": return "badge-skill";
    default: return "";
  }
}

function renderCards(items: DeeplinkItem[]) {
  if (items.length === 0) {
    cardList.innerHTML = '<div class="error">未找到任何配置<br><pre>请确保 CC Switch 已安装并至少添加过一个供应商</pre></div>';
    return;
  }

  cardList.innerHTML = items
    .map(
      (item, i) => `
    <div class="card" data-app="${escapeHtml(item.item_type === "skill" ? "skill" : item.app)}">
      <div class="card-head">
        <span class="badge ${badgeClass(item.item_type === "skill" ? "skill" : item.app)}">
          ${escapeHtml(item.item_type === "skill" ? "Skill" : item.app)}
        </span>
        <span class="card-name">${escapeHtml(item.name)}</span>
        ${item.is_current ? '<span class="card-current">当前</span>' : ""}
      </div>
      <div class="card-meta">
        ${item.endpoint ? `<span>端点: ${escapeHtml(item.endpoint)}</span>` : ""}
        ${item.model ? `<span>${item.item_type === "skill" ? "" : "模型: "}${escapeHtml(item.model)}</span>` : ""}
      </div>
      <div class="card-actions">
        <a href="${escapeHtml(item.deeplink)}" class="btn-import">导入到 CC Switch</a>
        <button class="btn-copy" data-idx="${i}">复制链接</button>
      </div>
      <details class="card-link">
        <summary>查看完整链接</summary>
        <div class="link-url">${escapeHtml(item.deeplink)}</div>
      </details>
    </div>
  `
    )
    .join("");

  // Bind copy buttons
  cardList.querySelectorAll<HTMLButtonElement>(".btn-copy").forEach((btn) => {
    btn.addEventListener("click", () => {
      const idx = parseInt(btn.dataset.idx!, 10);
      copyToClipboard(allItems[idx].deeplink);
    });
  });
}

function renderStats(items: DeeplinkItem[]) {
  const providers = items.filter((i) => i.item_type === "provider");
  const skills = items.filter((i) => i.item_type === "skill");
  const apps = new Set(providers.map((p) => p.app));

  statsEl.innerHTML = `
    <div class="stat"><div class="num">${providers.length}</div><div class="label">供应商</div></div>
    <div class="stat"><div class="num">${skills.length}</div><div class="label">Skills</div></div>
    <div class="stat"><div class="num">${apps.size}</div><div class="label">应用类型</div></div>
  `;
}

function filterItems(filter: string): DeeplinkItem[] {
  if (filter === "all") return allItems;
  if (filter === "skill") return allItems.filter((i) => i.item_type === "skill");
  return allItems.filter((i) => i.item_type === "provider" && i.app === filter);
}

async function loadData() {
  cardList.innerHTML = '<div class="loading">加载中...</div>';

  try {
    const dbPath = await invoke<string>("get_db_path_str");
    dbPathEl.textContent = dbPath;

    allItems = await invoke<DeeplinkItem[]>("load_deeplinks");
    renderStats(allItems);
    renderCards(allItems);
  } catch (e: any) {
    const msg = typeof e === "string" ? e : e?.message || JSON.stringify(e);
    cardList.innerHTML = `<div class="error">加载失败<pre>${escapeHtml(msg)}</pre></div>`;
  }
}

// Filter buttons
filterBtns.forEach((btn) => {
  btn.addEventListener("click", () => {
    filterBtns.forEach((b) => b.classList.remove("active"));
    btn.classList.add("active");
    const filter = btn.dataset.filter!;
    renderCards(filterItems(filter));
  });
});

// Copy all
btnCopyAll.addEventListener("click", () => {
  const links = allItems.map((i) => i.deeplink).join("\n");
  copyToClipboard(links);
});

// Export HTML
btnExport.addEventListener("click", () => {
  const html = generateExportHtml(allItems);
  const blob = new Blob([html], { type: "text/html" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = "ccswitch-deeplinks.html";
  a.click();
  URL.revokeObjectURL(url);
  showToast("已导出 HTML");
});

// Refresh
btnRefresh.addEventListener("click", () => {
  loadData();
});

function generateExportHtml(items: DeeplinkItem[]): string {
  const providers = items.filter((i) => i.item_type === "provider");
  const skills = items.filter((i) => i.item_type === "skill");
  const allLinksJson = JSON.stringify(items.map((i) => i.deeplink));
  const now = new Date().toLocaleString("zh-CN");

  const cards = items
    .map((item) => {
      const badgeCls = badgeClass(item.item_type === "skill" ? "skill" : item.app);
      const label = item.item_type === "skill" ? "Skill" : item.app;
      const current = item.is_current ? " <span style='color:#27ae60;font-size:11px;'>(当前)</span>" : "";
      return `
      <div class="card" data-app="${item.item_type === "skill" ? "skill" : item.app}">
        <div class="card-head">
          <span class="badge ${badgeCls}">${escapeHtml(label)}</span>
          <span class="card-name">${escapeHtml(item.name)}${current}</span>
        </div>
        <div class="card-meta">
          ${item.endpoint ? `<span>端点: ${escapeHtml(item.endpoint)}</span>` : ""}
          ${item.model ? `<span>${escapeHtml(item.model)}</span>` : ""}
        </div>
        <div class="card-actions">
          <a href="${escapeHtml(item.deeplink)}" class="btn-import">导入到 CC Switch</a>
          <button class="btn-copy" onclick="navigator.clipboard.writeText('${escapeHtml(item.deeplink.replace(/'/g, "\\'"))}').then(()=>{this.textContent='已复制!';setTimeout(()=>this.textContent='复制链接',1500)})">复制链接</button>
        </div>
        <details class="card-link">
          <summary>查看完整链接</summary>
          <div class="link-url">${escapeHtml(item.deeplink)}</div>
        </details>
      </div>`;
    })
    .join("");

  return `<!DOCTYPE html>
<html lang="zh-CN"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>CC Switch Deeplinks</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;background:#0f0f1a;color:#e8e8f0;min-height:100vh;padding:20px}
.container{max-width:900px;margin:0 auto}
h1{text-align:center;font-size:24px;background:linear-gradient(135deg,#00d2ff,#3a7bd5);-webkit-background-clip:text;-webkit-text-fill-color:transparent}
.sub{text-align:center;color:#7a7a8e;font-size:13px;margin:4px 0 20px}
.toolbar{display:flex;gap:6px;margin-bottom:16px;flex-wrap:wrap;justify-content:center}
.toolbar button{padding:6px 14px;border:1px solid rgba(255,255,255,0.08);border-radius:6px;background:transparent;color:#7a7a8e;cursor:pointer;font-size:12px;transition:all .15s}
.toolbar button:hover,.toolbar button.active{border-color:#00d2ff;color:#00d2ff;background:rgba(0,210,255,0.15)}
.btn-all{background:linear-gradient(135deg,#3a7bd5,#00d2ff)!important;color:white!important;border:none!important}
.card{background:rgba(255,255,255,0.04);border:1px solid rgba(255,255,255,0.08);border-radius:10px;padding:16px;margin-bottom:10px;transition:all .2s}
.card:hover{border-color:rgba(0,210,255,0.3)}
.card-head{display:flex;align-items:center;gap:10px;margin-bottom:6px}
.card-name{font-size:15px;font-weight:600}
.card-meta{display:flex;gap:20px;font-size:12px;color:#7a7a8e;margin-bottom:12px;flex-wrap:wrap}
.card-actions{display:flex;gap:8px;flex-wrap:wrap}
.badge{display:inline-block;padding:2px 8px;border-radius:4px;font-size:10px;font-weight:700;text-transform:uppercase}
.badge-claude{background:#1a3a5c;color:#5ba3d9}.badge-codex{background:#3d2e0a;color:#f39c12}.badge-gemini{background:#3d0a2a;color:#e91e63}.badge-skill{background:#0a3d2a;color:#2ecc71}
.btn-import{display:inline-flex;background:linear-gradient(135deg,#3a7bd5,#00d2ff);color:white;padding:7px 16px;border-radius:6px;text-decoration:none;font-size:12px;font-weight:600}
.btn-copy{background:linear-gradient(135deg,#9b59b6,#8e44ad);color:white;padding:7px 14px;border:none;border-radius:6px;cursor:pointer;font-size:12px}
.card-link{margin-top:10px}.card-link summary{font-size:11px;color:#4a4a5e;cursor:pointer}
.link-url{margin-top:6px;padding:10px;background:rgba(0,0,0,0.3);border-radius:6px;font-family:monospace;font-size:11px;color:#4a4a5e;word-break:break-all;border:1px solid rgba(255,255,255,0.04)}
.toast{position:fixed;top:20px;right:20px;background:#27ae60;color:white;padding:10px 20px;border-radius:8px;font-size:13px;font-weight:600;z-index:10000;transition:opacity .3s;pointer-events:none}
.hidden{opacity:0}
</style></head><body>
<div class="container">
<h1>CC Switch Deeplinks</h1>
<p class="sub">导出时间: ${now} | 共 ${providers.length} 个供应商, ${skills.length} 个 Skill</p>
<div class="toolbar">
<button class="btn-all" onclick="copyAll()">复制所有链接</button>
<button class="active" onclick="filter('all',this)">全部</button>
<button onclick="filter('claude',this)">Claude</button>
<button onclick="filter('codex',this)">Codex</button>
<button onclick="filter('gemini',this)">Gemini</button>
<button onclick="filter('skill',this)">Skills</button>
</div>
${cards}
</div>
<div id="toast" class="toast hidden"></div>
<script>
const allLinks=${allLinksJson};
function copyAll(){navigator.clipboard.writeText(allLinks.join('\\n')).then(()=>showToast('已复制 '+allLinks.length+' 个链接!'))}
function showToast(m){const t=document.getElementById('toast');t.textContent=m;t.classList.remove('hidden');setTimeout(()=>t.classList.add('hidden'),2000)}
function filter(app,btn){document.querySelectorAll('.toolbar button').forEach(b=>b.classList.remove('active'));btn.classList.add('active');document.querySelectorAll('.card').forEach(c=>{c.style.display=(app==='all'||c.dataset.app===app)?'':'none'})}
</script></body></html>`;
}

// Init
loadData();
