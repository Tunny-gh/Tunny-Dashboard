// Diagnostic only: execute the pinned official browser reader without modifying it.
const assert = require("node:assert/strict")
const fs = require("node:fs")
const path = require("node:path")

async function main() {
  const [upstream, typescript] = process.argv.slice(2)
  const ts = require(path.resolve(typescript))
  assert.equal(ts.version, "5.9.3")
  const source = fs.readFileSync(path.join(upstream, "tslib/storage/src/journal.ts"), "utf8")
  const output = ts.transpileModule(source, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 },
  }).outputText
  const { JournalFileStorage } = await import(
    `data:text/javascript;base64,${Buffer.from(output).toString("base64")}`
  )
  const exact = fs.readFileSync(path.join(__dirname, "../fixtures/issue258-terminal-attributes.jsonl"), "utf8")
  const load = (text) => new JournalFileStorage(new TextEncoder().encode(text).buffer)
  assert.throws(() => load(exact), TypeError)
  try { load(exact) } catch (error) {
    console.log(`Dashboard browser exact fixture: ${error.name}: ${error.message}`)
  }
  // Remove only the three explicit-null op8 records. Inline null and all other bytes remain.
  const withoutNullUpdates = exact.split("\n").filter((line) =>
    !line.includes('"user_attr":{"null":null}')
  ).join("\n")
  const reader = load(withoutNullUpdates)
  assert.equal(reader.getErrors().length, 0)
  for (const sid of [0, 1, 2, 3, 4, 5, 6, 7, 8]) {
    const study = await reader.getStudy(sid)
    console.log(JSON.stringify({ dashboard_browser_study: sid, trials: study.trials.map((trial) => ({
      id: trial.trial_id, number: trial.number, state: trial.state,
      user: trial.user_attrs, constraints: trial.constraints,
      artifactField: Object.hasOwn(trial, "artifacts"),
    })) }))
  }
  const complete = (await reader.getStudy(1)).trials[0]
  assert.deepEqual(complete.constraints, [1])
  assert.equal(complete.user_attrs.find((a) => a.key === "payload").value, "[object Object]")
  assert.equal(complete.user_attrs.find((a) => a.key === "empty").value, "")
  assert.deepEqual((await reader.getStudy(3)).trials[1].constraints, [-1, 0, 0])
  assert.deepEqual((await reader.getStudy(4)).trials[1].constraints, [-1, 0, 0, 0])
  assert.deepEqual((await reader.getStudy(2)).trials[0].constraints, [])
  const runningConstraints = (await reader.getStudy(6)).trials[1].constraints
  assert.ok(Number.isNaN(runningConstraints[2]))
  assert.equal(runningConstraints[3], Infinity)
  assert.equal(runningConstraints[4], -Infinity)

  for (const worker of ["probe", "other"]) {
    const scalar = [
      { op_code: 0, worker_id: "probe", study_name: "scalar", directions: [1] },
      { op_code: 4, worker_id: "probe", study_id: 0 },
      { op_code: 6, worker_id: "probe", trial_id: 0, state: 1, values: [1] },
      { op_code: 8, worker_id: worker, trial_id: 0, user_attr: { text: "late" } },
    ].map((record) => JSON.stringify(record)).join("\n")
    assert.deepEqual((await load(scalar).getStudy(0)).trials[0].user_attrs, [{ key: "text", value: "late" }])
  }
  console.log("Dashboard browser: late scalar attr accepted regardless of worker; late constraints applied; no Artifact association field")
  console.log("Dashboard browser diagnostic assertions passed (source transpilation, not browser UI/typecheck)")
}

main().catch((error) => { console.error(error); process.exitCode = 1 })
