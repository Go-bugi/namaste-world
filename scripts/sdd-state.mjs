#!/usr/bin/env node
/**
 * OpenSpec 변경이 SDD 상태 머신의 어느 칸에 있는지 판정한다. `sdd-next` 스킬이 가장 먼저 부른다.
 *
 *   node scripts/sdd-state.mjs [change]                     상태를 JSON으로 출력
 *   node scripts/sdd-state.mjs <change> --hash <종류>       산출물에 적을 해시 출력
 *
 * 해시 종류: spec(clarify.md 라운드 끝과 plan.md의 `spec-hash:`), approval(`.openspec.yaml`의 `x-approved:`),
 * plan(verify.md의 `plan-hash:`). 앞 단계 산출물이 바뀌면 해시가 달라져 그 단계로 돌아간다.
 *
 * change를 생략하면 진행 중인 변경이 하나일 때 그것을 고른다.
 * 판정 규칙은 아래 순수 함수(judge 등)에 있고, 파일과 CLI를 읽는 코드는 맨 아래 main에만 있다.
 */
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

// ---------------------------------------------------------------- 판정 (순수 함수)

/** clarify.md에서 `**답**:` 줄이 비어 있는 질문 수를 센다. */
export function openQuestions(text) {
  return text.split('\n').filter((line) => /^\*\*답\*\*:\s*$/.test(line)).length;
}

/**
 * 체크박스 작업 수와 완료 수를 센다. `x`(대소문자, 공백 무관)만 든 칸이 완료다.
 * 대괄호 안이 두 글자 이상인 줄(`- [위험] …`)은 체크박스가 아니라 목록 항목이라 세지 않는다.
 */
export function checklist(text) {
  const boxes = text
    .split('\n')
    .map((line) => /^\s*- \[([^\]]*)\]/.exec(line))
    .filter((box) => box && [...box[1].trim()].length <= 1);
  return {
    total: boxes.length,
    done: boxes.filter(([, mark]) => mark.trim().toLowerCase() === 'x').length,
  };
}

/** `.openspec.yaml`의 승인 표시(`x-approved: <승인 해시>`)가 지금 승인 해시와 같은지 본다. */
export function isApproved(meta, approvalHash) {
  return /^x-approved:\s*(\S+)\s*$/m.exec(meta)?.[1] === approvalHash;
}

/** `<key>: <해시>` 줄 가운데 마지막 것의 값. clarify는 라운드마다 덧붙이므로 마지막 라운드의 값이다. */
export function recordedHash(text, key) {
  const values = [...(text ?? '').matchAll(new RegExp(`^${key}:\\s*(\\S+)`, 'gm'))];
  return values.at(-1)?.[1];
}

/** plan.md에서 `## 작업` 앞까지. 체크박스 진행이나 verify가 덧붙인 수정 작업은 승인 해시에 넣지 않는다. */
export function designSection(plan) {
  const index = plan.search(/^## 작업/m);
  return index === -1 ? plan : plan.slice(0, index);
}

/** verify.md의 `결과:`와 `plan-hash:` 줄을 읽는다. */
export function parseVerify(text) {
  const result = /^결과:\s*(\S+)/m.exec(text);
  const hash = /^plan-hash:\s*(\S+)/m.exec(text);
  return { passed: result?.[1] === '통과', planHash: hash?.[1] };
}

/** delta나 기준 스펙에서 `### Requirement: FR-…`/`SC-…` ID를 delta 섹션(ADDED 등)과 함께 모은다. */
export function requirementIds(text) {
  const found = [];
  let op = 'ADDED';
  for (const line of text.split('\n')) {
    const section = /^## (ADDED|MODIFIED|REMOVED|RENAMED) Requirements/.exec(line);
    if (section) op = section[1];
    const req = /^### Requirement:\s*((?:FR|SC)-[A-Z]{2,6}-\d{3})\b/.exec(line);
    if (req) found.push({ op, id: req[1] });
  }
  return found;
}

/**
 * `openspec validate --strict --json` 결과에서 문제 문장을 뽑는다.
 * strict 검사는 통과해도 archive가 거부할 delta(없는 요구사항을 MODIFIED 등)는 INFO로만 나오므로 함께 문제로 본다.
 */
export function refusals(report) {
  return report.items.flatMap((item) =>
    item.issues
      .filter(
        (issue) =>
          (!item.valid && issue.level !== 'INFO') ||
          issue.message.startsWith('Archive would refuse'),
      )
      .map((issue) => issue.message),
  );
}

/**
 * 이 변경의 delta가 다른 진행 중인 변경이나 이미 쓴 ID와 겹치는지 찾는다.
 * ids.live는 지금 기준 스펙에 있는 ID, ids.everUsed는 기준 스펙과 archive에 한 번이라도 나온 ID다.
 */
export function findConflicts(current, others, ids) {
  const sameCapability = others.flatMap((other) =>
    current.capabilities
      .filter((cap) => other.capabilities.includes(cap))
      .map((cap) => `capability ${cap}: ${other.name}도 바꿉니다`),
  );
  const sameId = others.flatMap((other) =>
    other.ids
      .filter((theirs) => current.ids.some((mine) => mine.id === theirs.id))
      .map((theirs) => `${theirs.id}: ${other.name}도 ${theirs.op}합니다`),
  );
  const reused = current.ids
    .filter((mine) => mine.op === 'ADDED')
    .flatMap(({ id }) => {
      if (ids.live.includes(id)) return [`${id}: 이미 기준 스펙에 있습니다`];
      if (ids.everUsed.includes(id)) return [`${id}: 지워진 ID라 다시 쓸 수 없습니다`];
      return [];
    });
  return [...sameCapability, ...sameId, ...reused];
}

/**
 * 상태 표의 "넘어가는 조건"을 위에서부터 확인해, 처음 만족하지 못한 칸을 지금 상태로 정한다.
 * snapshot의 파일 값은 내용 문자열이고, 파일이 없으면 undefined다. hashes는 지금 산출물로 계산한 해시다.
 * 앞 단계 산출물이 바뀌면 그 뒤 단계에 적힌 해시와 달라져, 바뀐 곳 바로 다음 단계부터 다시 진행한다.
 */
export function judge(snapshot) {
  const { change, conflicts, hashes } = snapshot;
  const at = (state, missing) => ({ change, state, missing, conflicts });

  if (!snapshot.proposal) return at('proposal', 'proposal.md가 없습니다');
  if (!snapshot.validation.ok)
    return at('specs', `delta 검사 실패: ${snapshot.validation.problems.join(' / ')}`);
  if (snapshot.clarify === undefined) return at('clarify', 'clarify.md가 없습니다');
  const open = openQuestions(snapshot.clarify);
  if (open > 0) return at('clarify', `clarify.md에 답이 없는 질문 ${open}개`);
  if (recordedHash(snapshot.clarify, 'spec-hash') !== hashes.spec)
    return at('clarify', 'clarify 뒤에 delta가 바뀌었습니다');
  const tasks = snapshot.plan === undefined ? undefined : checklist(snapshot.plan);
  if (!tasks || tasks.total === 0) return at('plan', 'plan.md에 작업 체크리스트가 없습니다');
  if (recordedHash(snapshot.plan, 'spec-hash') !== hashes.spec)
    return at('plan', 'plan 뒤에 delta가 바뀌었습니다');
  if (!isApproved(snapshot.meta, hashes.approval))
    return at('approval', '지금 내용에 대한 사람의 승인(.openspec.yaml의 x-approved)이 없습니다');
  const testOpen = snapshot.testQuestions === undefined ? 0 : openQuestions(snapshot.testQuestions);
  if (testOpen > 0) return at('implement', `test-questions.md에 답이 없는 질문 ${testOpen}개`);
  if (tasks.done < tasks.total)
    return at('implement', `작업 ${tasks.total}개 중 ${tasks.total - tasks.done}개가 남았습니다`);
  if (snapshot.verify === undefined) return at('verify', 'verify.md가 없습니다');
  const verify = parseVerify(snapshot.verify);
  if (!verify.passed) return at('verify', 'verify.md 결과가 통과가 아닙니다');
  if (verify.planHash !== hashes.plan)
    return at('verify', 'verify 뒤에 plan.md가 바뀌어 다시 검증해야 합니다');
  return at('archive', '');
}

// ---------------------------------------------------------------- 파일과 CLI (shell)

const ROOT = join(fileURLToPath(import.meta.url), '..', '..');
const CHANGES = join(ROOT, 'openspec', 'changes');

const read = (path) => (existsSync(path) ? readFileSync(path, 'utf8') : undefined);
const dirs = (path) =>
  existsSync(path)
    ? readdirSync(path, { withFileTypes: true })
        .filter((d) => d.isDirectory())
        .map((d) => d.name)
    : [];

/** path 아래의 spec.md를 모두 찾아 { capability, text }로 돌려준다. */
function specFiles(path) {
  if (!existsSync(path)) return [];
  return readdirSync(path, { recursive: true })
    .filter((file) => file === 'spec.md' || file.endsWith(`${sep}spec.md`))
    .map((file) => ({
      capability: relative(path, join(path, file, '..'))
        .split(sep)
        .join('/'),
      text: readFileSync(join(path, file), 'utf8'),
    }));
}

/** 변경 하나의 delta 요약(capability 목록과 ID). */
function deltaOf(name, base = CHANGES) {
  const files = specFiles(join(base, name, 'specs'));
  return {
    name,
    capabilities: files.map((f) => f.capability),
    ids: files.flatMap((f) => requirementIds(f.text)),
  };
}

const hashOf = (...parts) =>
  createHash('sha256')
    .update(parts.map((part) => part ?? '').join('\n\u0000\n'))
    .digest('hex')
    .slice(0, 16);

/** 변경 하나의 해시들. spec은 delta 파일 전체, approval은 proposal + delta + plan 설계, plan은 plan.md 전체다. */
function hashesOf(change) {
  const dir = join(CHANGES, change);
  const specs = specFiles(join(dir, 'specs'))
    .sort((a, b) => a.capability.localeCompare(b.capability))
    .map((f) => `${f.capability}\n${f.text}`);
  const plan = read(join(dir, 'plan.md'));
  return {
    spec: hashOf(...specs),
    approval: hashOf(read(join(dir, 'proposal.md')), ...specs, designSection(plan ?? '')),
    plan: hashOf(plan),
  };
}

function validate(change) {
  const bin = join(ROOT, 'node_modules', '.bin', 'openspec');
  let out;
  try {
    out = execFileSync(bin, ['validate', change, '--strict', '--json'], {
      cwd: ROOT,
      env: { ...process.env, OPENSPEC_TELEMETRY: '0' },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
  } catch (error) {
    // 검사에 실패하면 종료 코드 1로 끝나지만 stdout에는 결과 JSON이 있다
    out = error.stdout;
  }
  const problems = refusals(JSON.parse(out.toString()));
  return { ok: problems.length === 0, problems };
}

function snapshotOf(change, active) {
  const dir = join(CHANGES, change);
  const archived = dirs(join(CHANGES, 'archive'));
  const live = specFiles(join(ROOT, 'openspec', 'specs')).flatMap((f) => requirementIds(f.text));
  const past = archived.flatMap((name) => deltaOf(name, join(CHANGES, 'archive')).ids);
  return {
    change,
    proposal: existsSync(join(dir, 'proposal.md')),
    validation: validate(change),
    clarify: read(join(dir, 'clarify.md')),
    plan: read(join(dir, 'plan.md')),
    meta: read(join(dir, '.openspec.yaml')) ?? '',
    testQuestions: read(join(dir, 'test-questions.md')),
    verify: read(join(dir, 'verify.md')),
    hashes: hashesOf(change),
    conflicts: findConflicts(
      deltaOf(change),
      active.filter((name) => name !== change).map((name) => deltaOf(name)),
      {
        live: live.map((r) => r.id),
        everUsed: [...live, ...past].map((r) => r.id),
      },
    ),
  };
}

function main(args) {
  const active = dirs(CHANGES).filter((name) => name !== 'archive');
  const positional = args.filter((arg, i) => !arg.startsWith('--') && args[i - 1] !== '--hash');
  const change = positional[0] ?? (active.length === 1 ? active[0] : undefined);

  const hashIndex = args.indexOf('--hash');
  if (hashIndex !== -1) {
    const kind = args[hashIndex + 1];
    if (!change || !active.includes(change))
      throw new Error('--hash에는 진행 중인 변경 이름이 필요합니다');
    const hashes = hashesOf(change);
    if (!(kind in hashes))
      throw new Error(`해시 종류는 ${Object.keys(hashes).join(', ')} 중 하나입니다`);
    console.log(hashes[kind]);
    return;
  }
  if (!change) {
    const state = active.length === 0 ? 'none' : 'choose';
    console.log(JSON.stringify({ state, changes: active }, null, 2));
    return;
  }
  if (!active.includes(change)) throw new Error(`진행 중인 변경이 아닙니다: ${change}`);
  console.log(JSON.stringify(judge(snapshotOf(change, active)), null, 2));
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main(process.argv.slice(2));
