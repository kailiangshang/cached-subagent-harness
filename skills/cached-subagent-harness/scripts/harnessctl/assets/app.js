(() => {
  "use strict";

  const copy = {
    "zh-CN": {
      productEdition: "结果控制台",
      currentRun: "当前运行",
      dataUpdated: "数据更新",
      connecting: "连接中",
      online: "实时连接",
      offline: "连接中断",
      runOutcome: "运行结果",
      deliveryProgress: "交付进度",
      acceptedTasks: "项任务已验收",
      effectiveTokens: "有效 Token",
      sessionReuse: "子 Agent 会话复用",
      acceptedFollowups: "已接受的续接任务",
      assignmentsPerSpawn: "每次子 Agent 启动任务数",
      churn: "Session 周转率",
      avoidedContext: "避免重复上下文",
      routingControl: "路由控制",
      dispatchPolicy: "调度策略",
      microBatch: "微批次上限",
      assignmentsMax: "最多 {count} 项",
      followupLimit: "续接上限",
      followupsMax: "最多 {count} 次",
      sessionTokenBudget: "复用资格 Token 上限",
      evidenceRequired: "提高上限需同质量精确用量证据",
      latestRoute: "最近路由",
      noRoute: "尚无路由决策",
      releasePolicy: "发布策略",
      howHarnessWorks: "Harness 如何工作",
      policyNotLive: "固定决策顺序 · 非当前任务轨迹",
      strategyIntake: "任务入口",
      strategyIntakeNote: "简单且无需隔离 → 主线程执行；否则评估委派价值",
      strategyShape: "兼容分组",
      strategyShapeNote: "严格匹配并保持顺序 · 每批最多 2 项",
      strategyRoute: "能力路由",
      strategyRouteNote: "先定质量与风险下限 · 再选最低合格模型",
      strategySession: "子 Agent 会话",
      strategySessionNote: "可证明且预算充足则续接 1 次 · 否则新建",
      strategyAccount: "完整成本与收口",
      strategyAccountNote: "统计全部 Token · 测试、审查、审计、关闭",
      workstream: "工作流",
      tasks: "任务",
      task: "任务",
      assignment: "执行归属",
      latestState: "最近状态",
      execution: "委派执行",
      sessions: "Session",
      subagentSessions: "子 Agent 会话",
      sessionDefinition: "Session 是子 Agent 的执行上下文，不是账户登录；列表包含当前与历史记录。",
      evidence: "消耗证据",
      tokenEconomy: "Token 经济性",
      costByPhase: "分阶段消耗",
      observedTotals: "观测总量",
      auditTrail: "审计轨迹",
      recentActivity: "最近动态",
      truthNotice: "事实来自本地 Harness 状态 · 缺失遥测保持未知",
      emptyTasks: "尚无任务数据",
      emptySessions: "尚无 Session 数据",
      emptyActivity: "尚无活动记录",
      unassigned: "未分配",
      currentTask: "当前任务",
      requestedModel: "请求模型",
      actualModel: "实际模型",
      routing: "路由",
      lastUsed: "最近使用",
      reuseTimes: "复用 {count} 次",
      estimateMethod: "按主机/配置的启动开销中位数",
      estimateUnavailable: "证据不足，暂不估算",
      samples: "{count} 个合格样本",
      input: "输入",
      output: "输出",
      reasoning: "推理",
      cacheRead: "缓存读取",
      cacheWrite: "缓存写入",
      status_active: "运行中",
      status_complete: "已完成",
      status_failed: "失败",
      status_cancelled: "已取消",
      status_queued: "排队中",
      status_running: "执行中",
      status_blocked: "阻塞",
      status_reported: "已汇报",
      status_accepted: "已验收",
      status_starting: "启动中",
      status_busy: "工作中",
      status_idle: "可复用",
      status_closed: "已关闭",
      status_unknown: "未知",
      quality_exact: "实测",
      quality_partial: "部分数据",
      quality_estimated: "估算",
      quality_unsupported: "不支持",
      quality_unknown: "未知",
      role_discussion: "讨论",
      role_explorer: "探索",
      role_worker: "开发",
      role_reviewer: "审查",
      role_fixer: "修复",
      profile_light: "轻量",
      profile_standard: "标准",
      profile_deep: "深度",
      phase_bootstrap: "启动",
      phase_context: "上下文",
      phase_work: "开发",
      phase_retry: "重试",
      phase_escalation: "升级",
      phase_review: "审查",
      phase_fixer: "修复",
      kind_plan: "已规划",
      kind_batch: "已合并",
      kind_spawn: "已启动子 Agent 会话",
      kind_reuse: "已接受复用",
      kind_route: "已完成路由",
      kind_start: "开始执行",
      kind_block: "出现阻塞",
      kind_report: "已提交结果",
      kind_accept: "已通过验收",
      kind_fail: "执行失败",
      kind_close: "生命周期关闭",
      legendAccepted: "已验收 {count}",
      legendActive: "进行中 {count}",
      legendBlocked: "需关注 {count}",
      legendQueued: "排队 {count}",
      percentComplete: "{value}% 已完成",
      ofTasks: "{accepted}/{total}",
      unknown: "未知"
    },
    "en-US": {
      productEdition: "Results Console",
      currentRun: "Current run",
      dataUpdated: "Data updated",
      connecting: "Connecting",
      online: "Live connection",
      offline: "Disconnected",
      runOutcome: "Run outcome",
      deliveryProgress: "Delivery progress",
      acceptedTasks: "tasks accepted",
      effectiveTokens: "Effective tokens",
      sessionReuse: "Subagent Session reuse",
      acceptedFollowups: "accepted follow-ups",
      assignmentsPerSpawn: "Tasks / Subagent spawn",
      churn: "Session churn",
      avoidedContext: "Avoided context",
      routingControl: "Routing control",
      dispatchPolicy: "Dispatch policy",
      microBatch: "Micro-batch limit",
      assignmentsMax: "Up to {count} tasks",
      followupLimit: "Follow-up limit",
      followupsMax: "Up to {count}",
      sessionTokenBudget: "Reuse eligibility Token cap",
      evidenceRequired: "Higher limits require equal-quality exact-usage evidence",
      latestRoute: "Latest route",
      noRoute: "No route decision yet",
      releasePolicy: "Release policy",
      howHarnessWorks: "How the Harness works",
      policyNotLive: "Fixed decision order · not the current Task trace",
      strategyIntake: "Task intake",
      strategyIntakeNote: "Simple, no isolation value → main; otherwise test delegation value",
      strategyShape: "Compatible shape",
      strategyShapeNote: "Strict match, declared order · at most 2 per batch",
      strategyRoute: "Capability route",
      strategyRouteNote: "Fix quality and risk floors · choose the lowest eligible model",
      strategySession: "Subagent Session",
      strategySessionNote: "Reuse once only with exact proof and budget · otherwise start new",
      strategyAccount: "Complete cost and closure",
      strategyAccountNote: "Count every Token · test, review, audit, close",
      workstream: "Workstream",
      tasks: "Tasks",
      task: "Task",
      assignment: "Assignment",
      latestState: "Latest state",
      execution: "Delegated execution",
      sessions: "Sessions",
      subagentSessions: "Subagent sessions",
      sessionDefinition: "A Session is a Subagent execution context, not an account login; the list includes current and historical records.",
      evidence: "Evidence",
      tokenEconomy: "Token economy",
      costByPhase: "Cost by phase",
      observedTotals: "Observed totals",
      auditTrail: "Audit trail",
      recentActivity: "Recent activity",
      truthNotice: "Facts from local Harness state · missing telemetry stays unknown",
      emptyTasks: "No task data yet",
      emptySessions: "No session data yet",
      emptyActivity: "No activity recorded yet",
      unassigned: "Unassigned",
      currentTask: "Current task",
      requestedModel: "Requested model",
      actualModel: "Actual model",
      routing: "Routing",
      lastUsed: "Last used",
      reuseTimes: "Reused {count} times",
      estimateMethod: "median overhead · host/profile",
      estimateUnavailable: "Insufficient evidence for an estimate",
      samples: "{count} eligible samples",
      input: "Input",
      output: "Output",
      reasoning: "Reasoning",
      cacheRead: "Cache read",
      cacheWrite: "Cache write",
      status_active: "Active",
      status_complete: "Complete",
      status_failed: "Failed",
      status_cancelled: "Cancelled",
      status_queued: "Queued",
      status_running: "Running",
      status_blocked: "Blocked",
      status_reported: "Reported",
      status_accepted: "Accepted",
      status_starting: "Starting",
      status_busy: "Busy",
      status_idle: "Reusable",
      status_closed: "Closed",
      status_unknown: "Unknown",
      quality_exact: "Exact",
      quality_partial: "Partial",
      quality_estimated: "Estimated",
      quality_unsupported: "Unsupported",
      quality_unknown: "Unknown",
      role_discussion: "Discussion",
      role_explorer: "Explorer",
      role_worker: "Worker",
      role_reviewer: "Reviewer",
      role_fixer: "Fixer",
      profile_light: "Light",
      profile_standard: "Standard",
      profile_deep: "Deep",
      phase_bootstrap: "Bootstrap",
      phase_context: "Context",
      phase_work: "Work",
      phase_retry: "Retry",
      phase_escalation: "Escalation",
      phase_review: "Review",
      phase_fixer: "Fixer",
      kind_plan: "Planned",
      kind_batch: "Batched",
      kind_spawn: "Subagent Session started",
      kind_reuse: "Reuse accepted",
      kind_route: "Route selected",
      kind_start: "Execution started",
      kind_block: "Blocked",
      kind_report: "Result reported",
      kind_accept: "Accepted",
      kind_fail: "Failed",
      kind_close: "Lifecycle closed",
      legendAccepted: "Accepted {count}",
      legendActive: "Active {count}",
      legendBlocked: "Attention {count}",
      legendQueued: "Queued {count}",
      percentComplete: "{value}% complete",
      ofTasks: "{accepted}/{total}",
      unknown: "Unknown"
    }
  };

  const defaultLanguage = navigator.language && navigator.language.startsWith("zh")
    ? "zh-CN"
    : "en-US";
  let language = localStorage.getItem("harness-language")
    || document.documentElement.lang
    || defaultLanguage;
  if (!copy[language]) language = defaultLanguage;

  let lastGoodSnapshot = null;
  let connectionState = "connecting";

  const el = id => document.getElementById(id);
  const clear = node => node.replaceChildren();
  const make = (tag, className, textValue) => {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (textValue !== undefined) node.textContent = textValue;
    return node;
  };
  const translate = (key, values = {}) => {
    let result = copy[language][key] || key;
    Object.entries(values).forEach(([name, value]) => {
      result = result.replace(`{${name}}`, String(value));
    });
    return result;
  };
  const known = value => value !== null && value !== undefined;
  const number = value => known(value) && Number.isFinite(Number(value))
    ? new Intl.NumberFormat(language).format(Number(value))
    : "—";
  const ratio = value => known(value) && Number.isFinite(Number(value))
    ? Number(value).toFixed(2)
    : "—";
  const date = value => {
    if (!value) return "—";
    const parsed = new Date(value);
    if (Number.isNaN(parsed.getTime())) return "—";
    return new Intl.DateTimeFormat(language, {
      month: "short",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit"
    }).format(parsed);
  };
  const stateText = value => translate(`status_${value || "unknown"}`);
  const qualityText = value => translate(`quality_${value || "unknown"}`);
  const roleText = value => translate(`role_${value || "discussion"}`);
  const profileText = value => translate(`profile_${value || "standard"}`);
  const phaseText = value => translate(`phase_${value || "work"}`);
  const kindText = value => translate(`kind_${value || "plan"}`);

  const progressOf = tasks => {
    const statuses = tasks.reduce((counts, task) => {
      counts[task.status] = (counts[task.status] || 0) + 1;
      return counts;
    }, {});
    const total = tasks.length;
    const accepted = statuses.accepted || 0;
    return {
      total,
      accepted,
      active: (statuses.running || 0) + (statuses.reported || 0),
      blocked: (statuses.blocked || 0) + (statuses.failed || 0),
      queued: statuses.queued || 0,
      percent: total ? Math.round((accepted / total) * 100) : 0
    };
  };

  const packagesOf = tasks => {
    const groups = new Map();
    tasks.forEach(task => {
      const key = task.package_key || translate("unknown");
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key).push(task);
    });
    return Array.from(groups, ([key, groupedTasks]) => ({ key, tasks: groupedTasks }));
  };

  const assignmentsFor = (session, tasks) =>
    tasks.filter(task => task.session_id === session.session_id);

  const latestFor = (task, activity) =>
    activity.find(item => item.task_id === task.task_id) || null;

  function badge(value, kind = "status") {
    const label = kind === "quality" ? qualityText(value) : stateText(value);
    const node = make("span", kind === "quality" ? "quality-tag" : "state-badge", label);
    node.dataset.state = value || "unknown";
    return node;
  }

  function empty(message) {
    return make("div", "empty-state", message);
  }

  function applyLanguage() {
    document.documentElement.lang = language;
    document.querySelectorAll("[data-i18n]").forEach(node => {
      node.textContent = translate(node.dataset.i18n);
    });
    el("language-current").textContent = language === "zh-CN" ? "中" : "EN";
    el("language-next").textContent = language === "zh-CN" ? "EN" : "中";
    el("language").setAttribute(
      "aria-label",
      language === "zh-CN" ? "Switch to English" : "切换到中文"
    );
    document.title = language === "zh-CN" ? "Token Harness · 结果" : "Token Harness · Results";
  }

  function setConnection(state) {
    connectionState = state;
    const health = el("health");
    health.dataset.state = state;
    const label = health.querySelector("span");
    label.textContent = translate(state);
  }

  function renderRun(data) {
    el("goal").textContent = data.run.goal;
    el("run-id").textContent = data.run.run_id;
    el("run-id").title = data.run.run_id;
    el("data-updated").textContent = date(data.run.updated_at);
    el("data-updated").dateTime = data.run.updated_at || "";
    el("run-status").replaceWith(badge(data.run.status));
    const replacement = document.querySelector(".run-heading-line .state-badge");
    replacement.id = "run-status";
  }

  function renderProgress(tasks) {
    const progress = progressOf(tasks);
    el("progress-value").textContent = translate("ofTasks", {
      accepted: progress.accepted,
      total: progress.total
    });
    el("progress-rate").textContent = translate("percentComplete", { value: progress.percent });

    const rail = el("progress-rail");
    clear(rail);
    rail.setAttribute(
      "aria-label",
      `${progress.accepted}/${progress.total} · ${progress.percent}%`
    );
    tasks.forEach(task => {
      const segment = make("span", "progress-segment");
      segment.dataset.state = task.status;
      segment.title = `${task.title} · ${stateText(task.status)}`;
      rail.appendChild(segment);
    });

    const legend = el("progress-legend");
    clear(legend);
    [
      ["accepted", "legendAccepted", progress.accepted],
      ["active", "legendActive", progress.active],
      ["blocked", "legendBlocked", progress.blocked],
      ["queued", "legendQueued", progress.queued]
    ].forEach(([state, key, count]) => {
      const item = make("span", "legend-item");
      item.dataset.state = state;
      item.append(make("i"), make("span", "", translate(key, { count })));
      legend.appendChild(item);
    });
  }

  function renderOutcomes(data) {
    const efficiency = data.efficiency;
    const totals = efficiency.totals;
    el("effective").textContent = number(totals.total_effective);
    el("reuse").textContent = number(efficiency.reuse_count);
    el("ratio").textContent = ratio(efficiency.assignments_per_spawn);
    el("churn").textContent = ratio(efficiency.churn_rate);
    el("saved").textContent = number(efficiency.estimated_saved_tokens);

    const quality = el("total-quality");
    quality.textContent = qualityText(totals.quality);
    quality.dataset.state = totals.quality || "unknown";

    const note = known(efficiency.estimated_saved_tokens)
      ? `${qualityText(efficiency.estimate_quality)} · ${translate("estimateMethod")} · ${translate("samples", { count: efficiency.estimate_sample_count })}`
      : `${translate("estimateUnavailable")} · ${translate("samples", { count: efficiency.estimate_sample_count })}`;
    el("estimate-note").textContent = note;
    el("estimate-note").title = note;
    renderProgress(data.tasks);
  }

  function renderPolicy(policy, activity) {
    const root = el("policy-facts");
    clear(root);
    const facts = [
      [translate("microBatch"), translate("assignmentsMax", {
        count: number(policy && policy.max_tasks_per_bundle)
      })],
      [translate("followupLimit"), translate("followupsMax", {
        count: number(policy && policy.max_accepted_followups)
      })],
      [translate("sessionTokenBudget"), number(policy && policy.max_effective_tokens)]
    ];
    facts.forEach(([label, value]) => {
      const fact = make("div", "policy-fact");
      fact.append(make("span", "", label), make("strong", "mono", value));
      root.appendChild(fact);
    });
    if (policy && policy.increases_require_evidence) {
      const note = make("p", "policy-note", translate("evidenceRequired"));
      root.appendChild(note);
    }

    const route = activity.find(item => item.kind === "route") || null;
    el("route-summary").textContent = route ? route.summary : translate("noRoute");
    el("route-meta").textContent = route
      ? `${route.task_id || route.session_id || "run"} · ${date(route.occurred_at)}`
      : "—";
  }

  function renderStrategy() {
    const root = el("strategy-steps");
    clear(root);
    [
      ["01", "strategyIntake", "strategyIntakeNote"],
      ["02", "strategyShape", "strategyShapeNote"],
      ["03", "strategyRoute", "strategyRouteNote"],
      ["04", "strategySession", "strategySessionNote"],
      ["05", "strategyAccount", "strategyAccountNote"]
    ].forEach(([index, title, note]) => {
      const item = make("li", "strategy-step");
      item.append(
        make("span", "strategy-index mono", index),
        make("strong", "", translate(title)),
        make("small", "", translate(note))
      );
      root.appendChild(item);
    });
  }

  function renderTasks(tasks, activity) {
    el("task-count").textContent = number(tasks.length);
    const root = el("task-packages");
    clear(root);
    if (!tasks.length) {
      root.appendChild(empty(translate("emptyTasks")));
      return;
    }

    packagesOf(tasks).forEach(group => {
      const section = make("section", "package-group");
      const head = make("header", "package-head");
      head.append(make("h3", "", group.key), make("span", "mono", number(group.tasks.length)));
      section.appendChild(head);

      group.tasks.forEach(task => {
        const row = make("article", "task-row");
        row.dataset.state = task.status;

        const main = make("div", "task-main");
        const titleLine = make("div", "task-title-line");
        const title = make("span", "task-title", task.title);
        title.title = task.title;
        titleLine.append(title, badge(task.status));
        const subline = make("div", "task-subline");
        subline.append(
          make("span", "mono", task.task_id),
          make("span", "", roleText(task.role)),
          make("span", "task-profile", profileText(task.required_profile))
        );
        main.append(titleLine, subline);

        const assignment = make("div", "task-assignment");
        const assignmentValue = task.session_id || translate("unassigned");
        const assignmentText = make("span", "mono", assignmentValue);
        assignmentText.title = assignmentValue;
        assignment.append(make("i", "tiny-node"), assignmentText);

        const latest = latestFor(task, activity);
        const latestNode = make("div", "task-latest");
        latestNode.append(
          make("i", "latest-mark"),
          make("span", "", latest ? kindText(latest.kind) : translate("unknown")),
          make("time", "mono", latest ? date(latest.occurred_at) : "—")
        );
        row.append(main, assignment, latestNode);
        section.appendChild(row);
      });
      root.appendChild(section);
    });
  }

  function modelRow(label, value) {
    const row = make("div", "session-model-row");
    const model = make("strong", "mono", value || "—");
    model.title = value || translate("unknown");
    row.append(make("span", "", label), model);
    return row;
  }

  function renderSessions(sessions, tasks) {
    el("session-count").textContent = number(sessions.length);
    const root = el("sessions");
    clear(root);
    if (!sessions.length) {
      root.appendChild(empty(translate("emptySessions")));
      return;
    }

    sessions.forEach(session => {
      const card = make("article", "session-card");
      const titleLine = make("div", "session-title-line");
      const identity = make("div", "session-identity");
      const id = make("strong", "mono", session.session_id);
      id.title = session.session_id;
      identity.append(
        id,
        make("div", "session-subline", `${session.host} · ${roleText(session.role)} · ${profileText(session.profile)}`)
      );
      titleLine.append(identity, badge(session.status));

      const models = make("div", "session-models");
      models.append(
        modelRow(translate("requestedModel"), session.requested_model),
        modelRow(translate("actualModel"), session.actual_model),
        modelRow(translate("routing"), session.routing_status)
      );

      const chain = make("div", "assignment-chain");
      const assignments = assignmentsFor(session, tasks);
      if (assignments.length) {
        assignments.forEach(task => {
          const node = make("div", "assignment-node");
          node.dataset.state = task.status;
          const name = make("span", "assignment-name", task.title);
          name.title = task.title;
          node.append(
            name,
            make("span", "assignment-meta", `${stateText(task.status)} · ${task.task_id}`)
          );
          chain.appendChild(node);
        });
      } else {
        chain.appendChild(make("span", "assignment-meta", translate("unassigned")));
      }

      const foot = make("div", "session-foot");
      const current = session.current_task_id
        ? `${translate("currentTask")}: ${session.current_task_id}`
        : `${translate("lastUsed")}: ${date(session.last_used_at)}`;
      const currentNode = make("span", "mono", current);
      currentNode.title = current;
      foot.append(
        currentNode,
        make("span", "reuse-chip", translate("reuseTimes", { count: session.reuse_count }))
      );
      card.append(titleLine, models, chain, foot);
      root.appendChild(card);
    });
  }

  function renderTokens(efficiency) {
    const totals = efficiency.totals;
    const composition = el("token-composition");
    clear(composition);
    [
      ["input", "input"],
      ["output", "output"],
      ["reasoning", "reasoning"],
      ["cache_read", "cacheRead"],
      ["cache_write", "cacheWrite"]
    ].forEach(([field, label]) => {
      const cell = make("div", "token-cell");
      cell.append(make("span", "", translate(label)), make("strong", "mono", number(totals[field])));
      composition.appendChild(cell);
    });

    const quality = el("token-quality");
    quality.textContent = qualityText(totals.quality);
    quality.dataset.state = totals.quality || "unknown";

    const phases = Array.isArray(efficiency.phase_totals) ? efficiency.phase_totals : [];
    const maximum = Math.max(
      1,
      ...phases.map(entry => Number(entry.totals.total_effective) || 0)
    );
    const phaseRoot = el("phase-totals");
    clear(phaseRoot);
    phases.forEach(entry => {
      const row = make("div", "phase-row");
      const bar = make("progress", "phase-bar");
      bar.max = maximum;
      bar.value = Number(entry.totals.total_effective) || 0;
      bar.setAttribute(
        "aria-label",
        `${phaseText(entry.phase)} · ${number(entry.totals.total_effective)}`
      );
      row.append(
        make("span", "phase-name", phaseText(entry.phase)),
        bar,
        make("span", "phase-value", number(entry.totals.total_effective)),
        badge(entry.totals.quality, "quality")
      );
      phaseRoot.appendChild(row);
    });
  }

  function renderActivity(activity) {
    const root = el("activity");
    clear(root);
    if (!activity.length) {
      root.appendChild(empty(translate("emptyActivity")));
      return;
    }
    activity.slice(0, 12).forEach(item => {
      const row = make("article", "activity-row");
      row.dataset.kind = item.kind;
      row.title = item.summary || kindText(item.kind);
      const copyNode = make("div", "activity-copy");
      copyNode.appendChild(make("strong", "", kindText(item.kind)));
      const meta = make("div", "activity-meta");
      const subject = item.task_id || item.session_id || "run";
      meta.append(
        make("span", "activity-summary", item.summary || translate("unknown")),
        make("span", "mono", subject)
      );
      copyNode.appendChild(meta);
      const time = make("time", "activity-time mono", date(item.occurred_at));
      time.dateTime = item.occurred_at || "";
      row.append(make("span", "activity-dot"), copyNode, time);
      root.appendChild(row);
    });
  }

  function render(data) {
    renderRun(data);
    renderOutcomes(data);
    renderPolicy(data.dispatch_policy, data.recent_activity);
    renderStrategy();
    renderTasks(data.tasks, data.recent_activity);
    renderSessions(data.sessions, data.tasks);
    renderTokens(data.efficiency);
    renderActivity(data.recent_activity);
    el("last-refresh").textContent = new Intl.DateTimeFormat(language, {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit"
    }).format(new Date());
  }

  async function refresh() {
    try {
      const response = await fetch("/api/status", { cache: "no-store" });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = await response.json();
      lastGoodSnapshot = data;
      render(data);
      setConnection("online");
    } catch (_error) {
      setConnection("offline");
    }
  }

  el("language").addEventListener("click", () => {
    language = language === "zh-CN" ? "en-US" : "zh-CN";
    localStorage.setItem("harness-language", language);
    applyLanguage();
    renderStrategy();
    setConnection(connectionState);
    if (lastGoodSnapshot) render(lastGoodSnapshot);
  });

  applyLanguage();
  renderStrategy();
  setConnection("connecting");
  refresh();
  window.setInterval(refresh, 1500);
})();
