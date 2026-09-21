import fs from "node:fs";

const html = fs.readFileSync("docs/design/ui-redesign-demo-v5.1.html", "utf8");
const product = fs.readFileSync(
  "../../src/views/XiaomiSettings.vue",
  "utf8",
);
let fail = 0;
const ok = (name, cond) => {
  console.log((cond ? "PASS " : "FAIL ") + name);
  if (!cond) fail++;
};

ok("title baseline", html.includes("对齐产品基线"));
ok("no primaryMap", !html.includes("primaryMap"));
ok("no bindCount", !html.includes("bindCount") && !html.includes("已绑定"));
ok("one-click copy", html.includes('点「一键修复」自动处理'));
ok("primary label fixed", html.includes('btn.textContent="一键修复"'));
ok("actions always in DOM", html.includes("map-card-actions") && html.includes("data-cap="));
ok("actions not gated by isSel only", !html.includes("const actions=isSel"));
ok("action bar always off", html.includes("setActionBarVisible(bar,false)"));
ok("repairing hides modal", html.includes('name!=="repairing"'));
ok("product primaryLabel one-click", product.includes('return "一键修复"'));
ok("product no action bar", product.includes("showActionBar = computed(() => false)"));
ok("product one-click desc", product.includes("点「一键修复」自动处理"));
ok("no 重试失败项", !html.includes("重试失败项"));
ok("no 分项主钮文案 in applyState", !html.includes("修复 ATVV 连接") || !html.includes("primaryMap"));

function layer(name) {
  const forceNoBlock = ["ready", "voice", "done_ok"].includes(name);
  const modal = !forceNoBlock && name !== "repairing";
  return { modal, bar: false };
}
for (const [n, em] of [
  ["ready", false],
  ["voice", false],
  ["done_ok", false],
  ["repairing", false],
  ["booting", true],
  ["error", true],
  ["err_bridge", true],
  ["idle", true],
  ["done_partial", true],
  ["done_fail", true],
]) {
  const r = layer(n);
  ok(`layer ${n} modal=${r.modal} bar=${r.bar}`, r.modal === em && r.bar === false);
}

console.log(fail === 0 ? "ALL PASS" : "FAILURES=" + fail);
process.exit(fail ? 1 : 0);
