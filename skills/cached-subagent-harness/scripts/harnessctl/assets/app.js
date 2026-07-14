(() => {
  const copy = {
    "zh-CN": { effective:"有效 Token", saved:"预估节省", reuse:"Session 复用", ratio:"每次 Spawn 任务数", tasks:"任务", agents:"Agent / Session", economy:"Token 经济性", activity:"最近动态", empty:"暂无数据", input:"输入", output:"输出", reasoning:"推理", cacheRead:"缓存读取", cacheWrite:"缓存写入" },
    "en-US": { effective:"Effective tokens", saved:"Estimated saved", reuse:"Session reuse", ratio:"Assignments per spawn", tasks:"Tasks", agents:"Agents / Sessions", economy:"Token economy", activity:"Recent activity", empty:"No data", input:"Input", output:"Output", reasoning:"Reasoning", cacheRead:"Cache read", cacheWrite:"Cache write" }
  };
  let language = localStorage.getItem("harness-language") || document.documentElement.lang || "zh-CN";
  if (!copy[language]) language = "zh-CN";
  const el = id => document.getElementById(id);
  const value = v => v === null || v === undefined ? "—" : Number.isFinite(v) ? new Intl.NumberFormat(language).format(v) : String(v);
  const clear = node => { while (node.firstChild) node.removeChild(node.firstChild); };
  const text = (tag, className, content) => { const node = document.createElement(tag); node.className = className; node.textContent = content; return node; };
  function applyLanguage() { document.documentElement.lang = language; document.querySelectorAll("[data-i18n]").forEach(node => { node.textContent = copy[language][node.dataset.i18n]; }); }
  function empty(node) { node.appendChild(text("p", "empty", copy[language].empty)); }
  function renderRows(node, rows, kind) {
    clear(node); if (!rows.length) return empty(node);
    rows.forEach(item => {
      const row = text("div", "row", "");
      if (kind === "task") { row.append(text("span","id",item.task_id), text("span","pill",item.status), text("span","meta",item.title), text("span","meta",item.required_profile)); }
      if (kind === "agent") { row.append(text("span","id",item.session_id), text("span","pill",item.status), text("span","meta",`${item.host} · ${item.role} · ${item.profile} · ${item.actual_model || "—"} · ${item.current_task_id || "—"}`), text("span","meta",`×${item.reuse_count} · ${item.last_used_at}`)); }
      if (kind === "activity") { row.append(text("span","id",item.kind), text("span","meta",item.summary), text("span","meta",item.task_id || "—"), text("span","meta",item.occurred_at)); }
      node.appendChild(row);
    });
  }
  function renderEconomy(totals) {
    const node = el("economy"); clear(node);
    [["input",copy[language].input],["output",copy[language].output],["reasoning",copy[language].reasoning],["cache_read",copy[language].cacheRead],["cache_write",copy[language].cacheWrite]].forEach(([key,label]) => {
      const card = text("div","token",""); card.append(text("span","",label), text("strong","",value(totals[key]))); node.appendChild(card);
    });
    [["Churn", state.efficiency.churn_rate],["Estimate samples", state.efficiency.estimate_sample_count],["Estimate quality", state.efficiency.estimate_quality]].forEach(([label,number]) => { const card = text("div","token",""); card.append(text("span","",label), text("strong","",value(number))); node.appendChild(card); });
  }
  let state = null;
  function render(data) {
    state = data;
    el("goal").textContent = data.run.goal;
    el("effective").textContent = value(data.efficiency.totals.total_effective);
    el("saved").textContent = value(data.efficiency.estimated_saved_tokens);
    el("reuse").textContent = value(data.efficiency.reuse_count);
    el("ratio").textContent = data.efficiency.assignments_per_spawn == null ? "—" : data.efficiency.assignments_per_spawn.toFixed(2);
    el("task-count").textContent = value(data.tasks.length); el("agent-count").textContent = value(data.sessions.length);
    el("quality").textContent = data.efficiency.totals.quality; el("updated").textContent = new Date().toLocaleTimeString(language);
    renderRows(el("tasks"), data.tasks, "task"); renderRows(el("agents"), data.sessions, "agent"); renderRows(el("activity"), data.recent_activity, "activity"); renderEconomy(data.efficiency.totals);
  }
  async function refresh() { try { const response = await fetch("/api/status", {cache:"no-store"}); if (!response.ok) throw new Error(String(response.status)); render(await response.json()); el("health").textContent = "● LIVE"; } catch (_) { el("health").textContent = "● OFFLINE"; } }
  el("language").addEventListener("click", () => { language = language === "zh-CN" ? "en-US" : "zh-CN"; localStorage.setItem("harness-language", language); applyLanguage(); refresh(); });
  applyLanguage(); refresh(); setInterval(refresh, 1500);
})();
