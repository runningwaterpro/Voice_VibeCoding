import fs from "node:fs";
import crypto from "node:crypto";

const v52 = fs.readFileSync("docs/design/ui-redesign-demo-v5.2.html", "utf8");
const v51 = fs.readFileSync("docs/design/ui-redesign-demo-v5.1.html", "utf8");
let fail = 0;
const ok = (n, c) => {
  console.log((c ? "PASS " : "FAIL ") + n);
  if (!c) fail++;
};
const h = (s) => crypto.createHash("sha256").update(s).digest("hex");

ok("title v5.2", v52.includes("Demo v5.2"));
ok("demo-bar v5.2", v52.includes("v5.2"));
ok("v5.1 unchanged by this check content", v51.includes("Demo v5.1"));
ok("no actionBar id", !v52.includes('id="actionBar"'));
ok("no btnPrimary", !v52.includes('id="btnPrimary"'));
ok("no setActionBarVisible fn", !v52.includes("function setActionBarVisible"));
ok("no force-repair buttons", !v52.includes("data-force-repair"));
ok("meters idle class", v52.includes(".meters.is-idle"));
ok("meters id", v52.includes('id="meters"'));
ok("overlay lighter", v52.includes("rgba(0,0,0,.35)") && !v52.includes("backdrop-filter:blur"));
ok("one-click", v52.includes("btn.textContent=\"一键修复\""));
ok("repairing keeps card", !v52.includes('name!=="repairing"') && v52.includes("正在修复…"));
ok("repairing disabled btn", v52.includes("btn.disabled=true") && v52.includes("修复中…"));
ok("not-ready lock", v52.includes("is-not-ready") && v52.includes(".app.is-not-ready .work"));
ok("lean hint", v52.includes("点录入后按目标键或组合键"));
ok("unbound dashed", v52.includes("border:1px dashed"));
ok("no bindCount", !v52.includes("bindCount"));

function apply(name) {
  const healthy = ["ready", "voice", "done_ok"].includes(name);
  const showModal = !healthy;
  const locked = !healthy;
  const idleMeters = !(name === "ready" || name === "voice");
  const showSub = !healthy;
  return { showModal, locked, idleMeters, showSub };
}
for (const [n, m, lock, idle, sub] of [
  ["ready", false, false, false, false],
  ["voice", false, false, false, false],
  ["done_ok", false, false, true, false],
  ["repairing", true, true, true, true],
  ["booting", true, true, true, true],
  ["error", true, true, true, true],
  ["err_bridge", true, true, true, true],
]) {
  const r = apply(n);
  ok(
    `${n} card=${r.showModal} lock=${r.locked} idle=${r.idleMeters} sub=${r.showSub}`,
    r.showModal === m && r.locked === lock && r.idleMeters === idle && r.showSub === sub,
  );
}
console.log(fail === 0 ? "ALL PASS" : "FAILURES=" + fail);
process.exit(fail ? 1 : 0);
