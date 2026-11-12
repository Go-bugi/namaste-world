import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import {
  checklist,
  designSection,
  findConflicts,
  isApproved,
  judge,
  openQuestions,
  parseVerify,
  recordedHash,
  refusals,
  requirementIds,
} from './sdd-state.mjs';

const PLAN_DONE =
  '# Plan\n\nspec-hash: s1\n\n## 작업\n\n### 1. 그룹\n\n- [x] 1.1 하나 (FR-AGENT-001)\n- [X] 1.2 둘 (FR-AGENT-001)\n';
const PLAN_OPEN = '# Plan\n\nspec-hash: s1\n\n## 작업\n\n- [x] 1.1 하나\n- [ ] 1.2 둘\n';
const CLARIFY_DONE = '## 질문\n\n### Q1. 시간 제한은?\n\n**답**: 10초\n\nspec-hash: s1\n';
const VERIFY_PASS = (hash) => `결과: 통과\nplan-hash: ${hash}\n`;

/** 모든 조건을 만족한 snapshot. 테스트마다 하나씩 깨뜨려 상태를 확인한다. */
const complete = () => ({
  change: 'add-x',
  proposal: true,
  validation: { ok: true, problems: [] },
  clarify: CLARIFY_DONE,
  plan: PLAN_DONE,
  meta: 'schema: namaste\nx-approved: a1\n',
  testQuestions: undefined,
  verify: VERIFY_PASS('abc'),
  hashes: { spec: 's1', approval: 'a1', plan: 'abc' },
  conflicts: [],
});

describe('openQuestions', () => {
  it('답 줄이 비어 있는 질문만 센다', () => {
    const text = '### Q1. a\n\n**답**: 예\n\n### Q2. b\n\n**답**:\n\n### Q3. c\n\n**답**:   \n';
    assert.equal(openQuestions(text), 2);
  });

  it('질문 없음이면 0이다', () => {
    assert.equal(openQuestions('## 질문\n\n질문 없음: 스펙이 단순함\n'), 0);
  });
});

describe('checklist', () => {
  it('OpenSpec apply와 같게, x만 든 칸을 완료로 세고 다른 표시는 남은 작업으로 센다', () => {
    const text = '- [x] a\n- [ X ] b\n- [ ] c\n- [~] d\n- [] e\n그냥 줄\n';
    assert.deepEqual(checklist(text), { total: 5, done: 2 });
  });

  it('대괄호 안이 두 글자 이상인 목록 항목(plan의 위험)은 세지 않는다', () => {
    const text = '- [ ] 1.1 작업\n- [x] 1.2 작업\n- [위험] 느림 → 제한 시간\n- [TODO] 메모\n';
    assert.deepEqual(checklist(text), { total: 2, done: 1 });
  });
});

describe('isApproved', () => {
  it('x-approved 값이 지금 승인 해시와 같을 때만 승인으로 본다', () => {
    assert.equal(isApproved('schema: namaste\nx-approved: a1\n', 'a1'), true);
    assert.equal(isApproved('schema: namaste\nx-approved: a0\n', 'a1'), false);
    assert.equal(isApproved('schema: namaste\nx-approved: true\n', 'a1'), false);
    assert.equal(isApproved('schema: namaste\n', 'a1'), false);
  });
});

describe('recordedHash', () => {
  it('마지막 spec-hash 줄을 읽는다(clarify는 라운드마다 덧붙인다)', () => {
    const text = '## 라운드 1\n\nspec-hash: old\n\n## 라운드 2\n\nspec-hash: new\n';
    assert.equal(recordedHash(text, 'spec-hash'), 'new');
  });

  it('줄이 없거나 파일이 없으면 undefined다', () => {
    assert.equal(recordedHash('# Plan\n', 'spec-hash'), undefined);
    assert.equal(recordedHash(undefined, 'spec-hash'), undefined);
  });
});

describe('designSection', () => {
  it('## 작업 앞까지만 돌려줘 체크박스가 바뀌어도 승인 해시가 그대로다', () => {
    const before = '# Plan\n\n## 설계\n\nA\n\n## 작업\n\n- [ ] 1.1\n';
    const after = '# Plan\n\n## 설계\n\nA\n\n## 작업\n\n- [x] 1.1\n- [ ] 9.1 수정\n';
    assert.equal(designSection(before), designSection(after));
    assert.equal(designSection(before), '# Plan\n\n## 설계\n\nA\n\n');
  });

  it('## 작업이 없으면 전체가 설계다', () => {
    assert.equal(designSection('# Plan\n\nA\n'), '# Plan\n\nA\n');
  });
});

describe('parseVerify', () => {
  it('결과와 plan-hash를 읽는다', () => {
    assert.deepEqual(parseVerify('# Verify\n\n결과: 실패\nplan-hash: 1f2e\n'), {
      passed: false,
      planHash: '1f2e',
    });
    assert.deepEqual(parseVerify(VERIFY_PASS('9a')), { passed: true, planHash: '9a' });
  });
});

describe('requirementIds', () => {
  it('delta 섹션별로 FR/SC ID를 모은다', () => {
    const text = [
      '## ADDED Requirements',
      '### Requirement: FR-AGENT-001 감지',
      '### Requirement: SC-AGENT-001 10초',
      '## MODIFIED Requirements',
      '### Requirement: FR-TRAY-002 메뉴',
      '## REMOVED Requirements',
      '### Requirement: FR-TRAY-003 옛 메뉴',
      '### Requirement: ID 없는 요구사항',
    ].join('\n');
    assert.deepEqual(requirementIds(text), [
      { op: 'ADDED', id: 'FR-AGENT-001' },
      { op: 'ADDED', id: 'SC-AGENT-001' },
      { op: 'MODIFIED', id: 'FR-TRAY-002' },
      { op: 'REMOVED', id: 'FR-TRAY-003' },
    ]);
  });
});

describe('refusals', () => {
  it('strict 검사 실패와 archive가 거부할 delta를 모두 문제로 본다', () => {
    const report = {
      items: [
        {
          valid: true,
          issues: [
            { level: 'INFO', message: 'Archive would refuse this delta: agent MODIFIED failed' },
            { level: 'INFO', message: '참고용 정보' },
          ],
        },
      ],
    };
    assert.deepEqual(refusals(report), ['Archive would refuse this delta: agent MODIFIED failed']);
  });

  it('valid가 false면 WARNING과 ERROR를 문제로 본다', () => {
    const report = {
      items: [{ valid: false, issues: [{ level: 'ERROR', message: '시나리오 없음' }] }],
    };
    assert.deepEqual(refusals(report), ['시나리오 없음']);
  });
});

describe('findConflicts', () => {
  const current = {
    name: 'add-x',
    capabilities: ['agent'],
    ids: [
      { op: 'ADDED', id: 'FR-AGENT-004' },
      { op: 'MODIFIED', id: 'FR-AGENT-001' },
    ],
  };

  it('다른 진행 중인 변경이 같은 capability와 같은 ID를 건드리면 알린다', () => {
    const others = [
      { name: 'add-y', capabilities: ['agent'], ids: [{ op: 'MODIFIED', id: 'FR-AGENT-001' }] },
      { name: 'add-z', capabilities: ['tray'], ids: [] },
    ];
    assert.deepEqual(findConflicts(current, others, { live: [], everUsed: [] }), [
      'capability agent: add-y도 바꿉니다',
      'FR-AGENT-001: add-y도 MODIFIED합니다',
    ]);
  });

  it('ADDED ID가 기준 스펙에 이미 있거나 지워진 ID면 알린다', () => {
    assert.deepEqual(
      findConflicts(current, [], {
        live: ['FR-AGENT-001'],
        everUsed: ['FR-AGENT-001', 'FR-AGENT-004'],
      }),
      ['FR-AGENT-004: 지워진 ID라 다시 쓸 수 없습니다'],
    );
    assert.deepEqual(
      findConflicts(current, [], { live: ['FR-AGENT-004'], everUsed: ['FR-AGENT-004'] }),
      ['FR-AGENT-004: 이미 기준 스펙에 있습니다'],
    );
  });
});

describe('judge', () => {
  it('모든 조건을 만족하면 archive다', () => {
    assert.equal(judge(complete()).state, 'archive');
  });

  it('조건을 위에서부터 확인해 처음 깨진 칸을 고른다', () => {
    const cases = [
      [{ proposal: false }, 'proposal'],
      [{ validation: { ok: false, problems: ['시나리오 없음'] } }, 'specs'],
      [{ clarify: undefined }, 'clarify'],
      [{ clarify: '### Q1. a\n\n**답**:\n' }, 'clarify'],
      [{ plan: undefined }, 'plan'],
      [{ plan: '# Plan\n\nspec-hash: s1\n\n## 작업\n\n아직 없음\n' }, 'plan'],
      [{ meta: 'schema: namaste\n' }, 'approval'],
      [{ testQuestions: '### TQ1. 경계값은?\n\n**답**:\n' }, 'implement'],
      [{ plan: PLAN_OPEN }, 'implement'],
      [{ verify: undefined }, 'verify'],
      [{ verify: '결과: 실패\nplan-hash: abc\n' }, 'verify'],
      [{ hashes: { spec: 's1', approval: 'a1', plan: 'changed' } }, 'verify'],
    ];
    for (const [patch, state] of cases) {
      assert.equal(judge({ ...complete(), ...patch }).state, state, JSON.stringify(patch));
    }
  });

  it('승인 전에는 체크리스트가 남아 있어도 implement로 넘어가지 않는다', () => {
    const result = judge({ ...complete(), meta: 'schema: namaste\n', plan: PLAN_OPEN });
    assert.equal(result.state, 'approval');
  });

  it('승인 뒤 구현 중이어도 clarify를 다시 해서 답이 빈 질문이 생기면 clarify로 돌아간다', () => {
    const reopened = `${CLARIFY_DONE}\n## 질문 (라운드 2)\n\n### Q2. 실패하면?\n\n**답**:\n`;
    const result = judge({ ...complete(), plan: PLAN_OPEN, clarify: reopened });
    assert.equal(result.state, 'clarify');
  });

  it('clarify 뒤에 delta가 바뀌면 바뀐 부분을 묻도록 clarify로 돌아간다', () => {
    const result = judge({ ...complete(), hashes: { spec: 's2', approval: 'a1', plan: 'abc' } });
    assert.equal(result.state, 'clarify');
    assert.equal(result.missing, 'clarify 뒤에 delta가 바뀌었습니다');
  });

  it('clarify가 바뀐 delta를 다시 확인하면 plan으로 넘어가 plan을 고친다', () => {
    const result = judge({
      ...complete(),
      clarify: `${CLARIFY_DONE}\n## 질문 (라운드 2)\n\n질문 없음: 문구만 바뀜\n\nspec-hash: s2\n`,
      hashes: { spec: 's2', approval: 'a1', plan: 'abc' },
    });
    assert.equal(result.state, 'plan');
    assert.equal(result.missing, 'plan 뒤에 delta가 바뀌었습니다');
  });

  it('plan까지 바뀐 delta에 맞춘 뒤에는 승인을 다시 받는다', () => {
    const result = judge({
      ...complete(),
      clarify: `${CLARIFY_DONE}\nspec-hash: s2\n`,
      plan: PLAN_DONE.replace('spec-hash: s1', 'spec-hash: s2'),
      hashes: { spec: 's2', approval: 'a2', plan: 'abc' },
    });
    assert.equal(result.state, 'approval');
  });

  it('test-writer 질문에 답이 없으면 작업을 다 끝내도 implement에 머문다', () => {
    const result = judge({ ...complete(), testQuestions: '### TQ1. a\n\n**답**:\n' });
    assert.equal(result.state, 'implement');
    assert.equal(result.missing, 'test-questions.md에 답이 없는 질문 1개');
  });

  it('부족한 것과 충돌을 함께 돌려준다', () => {
    const result = judge({
      ...complete(),
      clarify: '### Q1. a\n\n**답**:\n\n### Q2. b\n\n**답**:\n',
      conflicts: ['FR-AGENT-001: add-y도 MODIFIED합니다'],
    });
    assert.deepEqual(result, {
      change: 'add-x',
      state: 'clarify',
      missing: 'clarify.md에 답이 없는 질문 2개',
      conflicts: ['FR-AGENT-001: add-y도 MODIFIED합니다'],
    });
  });
});
