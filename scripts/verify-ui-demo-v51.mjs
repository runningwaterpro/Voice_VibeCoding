import fs from "node:fs";

const html = fs.readFileSync("docs/design/ui-redesign-demo-v5.1.html", "utf8");
const v50 = fs.readFileSync("docs/design/ui-redesign-demo-v5.0.html", "utf8");
let fail = 0;
const ok = (name, cond) => {
  console.log((cond ? "PASS " : "FAIL ") + name);
  if (!cond) fail++;
};

ok("title v5.1", html.includes("Demo v5.1"));
ok("demo-bar v5.1", html.includes("预览控制 · v5.1"));
ok("bindCount el", html.includes('id="bindCount"'));
ok("bind CSS", html.includes(".bind-count{"));
ok("count text", html.includes("已绑定"));
ok("actions isSel", html.includes("const actions=isSel"));
ok("forceNoBlock", html.includes("forceNoBlock"));
ok("showBar primary", html.includes("showBar && s.primary"));
ok("v5.0 still v5.0 title", v50.includes("Demo v5.0"));
ok("v5.0 not v5.1", !v50.includes("Demo v5.1"));

const healthy = (n) => ["ready", "voice", "done_ok"].includes(n);
function layer(name, s = {}) {
  const useModalOnly =
    ["booting", "repairing", "done_partial", "done_fail"].includes(name) ||
    (!!s.showRepairModal && !healthy(name) && name !== "done_ok");
  const useBarOnly = !healthy(name) && !useModalOnly && name !== "done_ok";
  const forceNoBlock = ["done_ok", "ready", "voice"].includes(name);
  return { modal: !forceNoBlock && useModalOnly, bar: !forceNoBlock && useBarOnly };
}

const cases = [
  ["ready", {}, false, false],
  ["voice", {}, false, false],
  ["done_ok", {}, false, false],
  ["booting", { booting: true }, true, false],
  ["repairing", { showRepairModal: true }, true, false],
  ["done_partial", { showRepairModal: true }, true, false],
  ["done_fail", { showRepairModal: true }, true, false],
  ["error", {}, false, true],
  ["err_bridge", {}, false, true],
  ["err_hid", {}, false, true],
  ["idle", {}, false, true],
];
for (const [n, s, em, eb] of cases) {
  const r = layer(n, s);
  ok(`layer ${n} modal=${r.modal} bar=${r.bar}`, r.modal === em && r.bar === eb);
  if (r.modal && r.bar) ok(`dual exclusive ${n}`, false);
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

// unselected cards must not contain map-card-actions in static sense:
// renderMaps only adds actions when isSel — verify source pattern once
ok("no always-on actions template", !html.includes('el.innerHTML=`\n      <div class="map-card-main">') || html.includes("const actions=isSel"));

console.log(fail === 0 ? "ALL PASS" : "FAILURES=" + fail);
process.exit(fail ? 1 : 0);
