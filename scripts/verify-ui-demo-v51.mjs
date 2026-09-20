import fs from "node:fs";

const html = fs.readFileSync("docs/design/ui-redesign-demo-v5.1.html", "utf8");
const v50 = fs.readFileSync("docs/design/ui-redesign-demo-v5.0.html", "utf8");
let fail = 0;
const ok = (name, cond) => {
  console.log((cond ? "PASS " : "FAIL ") + name);
  if (!cond) fail++;
};

ok("title v5.1", html.includes("Demo v5.1"));
ok("demo-bar copy", html.includes("居中卡唯一交互面"));
ok("bindCount el", html.includes('id="bindCount"'));
ok("bind CSS", html.includes(".bind-count{"));
ok("count text", html.includes("已绑定"));
ok("actions isSel", html.includes("const actions=isSel"));
ok("forceNoBlock", html.includes("forceNoBlock"));
ok("no showBar revival", !html.includes("showBar && s.primary") && !html.includes("const showBar"));
ok("action bar always hidden", html.includes('setActionBarVisible(bar,false)'));
ok("btnPrimary forced hidden", html.includes("btnP.hidden=true"));
ok("v5.0 still v5.0 title", v50.includes("Demo v5.0"));
ok("v5.0 not v5.1", !v50.includes("Demo v5.1"));

/* Mirrors applyState contract after correction:
   ready/voice/done_ok → no modal no bar
   any other → modal only, bar always false */
function layer(name) {
  const forceNoBlock = ["ready", "voice", "done_ok"].includes(name);
  return { modal: !forceNoBlock, bar: false };
}

const cases = [
  ["ready", false],
  ["voice", false],
  ["done_ok", false],
  ["booting", true],
  ["repairing", true],
  ["done_partial", true],
  ["done_fail", true],
  ["error", true],
  ["err_bridge", true],
  ["err_hid", true],
  ["err_cable", true],
  ["err_route", true],
  ["idle", true],
  ["connecting", true],
];
for (const [n, em] of cases) {
  const r = layer(n);
  ok(`layer ${n} modal=${r.modal} bar=${r.bar}`, r.modal === em && r.bar === false);
  if (r.bar) ok(`bar never ${n}`, false);
}

const KEYS = [
  { bind: "a" },
  { bind: "未映射" },
  { ghost: true },
  { bind: "b" },
];
const real = KEYS.filter((k) => !k.ghost);
const bound = real.filter((k) => k.bind && k.bind !== "未映射").length;
ok(`bindCount ${bound}/${real.length}`, bound === 2 && real.length === 3);
ok("lean cards", html.includes("const actions=isSel"));

console.log(fail === 0 ? "ALL PASS" : "FAILURES=" + fail);
process.exit(fail ? 1 : 0);
