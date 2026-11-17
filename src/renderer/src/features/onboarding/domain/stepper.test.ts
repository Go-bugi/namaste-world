import { fc, test } from '@fast-check/vitest';
import { describe, expect, it } from 'vitest';
import {
  indicators,
  navigation,
  nextStep,
  prevStep,
  stepsFor,
  type StepId,
  type StepperInput,
} from './stepper';

/** macOS 단계 순서 */
const MAC: readonly StepId[] = ['detect', 'install', 'select', 'login'];
/** Windows 단계 순서. 셸 선택이 맨 앞이다 */
const WIN: readonly StepId[] = ['shell', 'detect', 'install', 'select', 'login'];

/** macOS에서 감지와 설치가 끝났고 아무것도 진행 중이 아닌 입력 */
const base: StepperInput = {
  steps: MAC,
  current: 'detect',
  shellConfirmed: false,
  detecting: false,
  hasInstalled: true,
  installing: false,
  hasInstallation: true,
  selectionReady: false,
  loggedIn: false,
  submitting: false,
};

const at = (current: StepId, patch: Partial<StepperInput> = {}): StepperInput => ({
  ...base,
  ...patch,
  current,
});

/** Windows에서 셸을 확정했고 감지와 설치가 끝났으며 아무것도 진행 중이 아닌 입력 */
const winAt = (current: StepId, patch: Partial<StepperInput> = {}): StepperInput =>
  at(current, { steps: WIN, shellConfirmed: true, ...patch });

const statusOf = (input: StepperInput, id: StepId) =>
  indicators(input).find((indicator) => indicator.id === id);

const stepsArb = fc.constantFrom(MAC, WIN);

const inputArb: fc.Arbitrary<StepperInput> = stepsArb.chain((steps) =>
  fc.record({
    steps: fc.constant(steps),
    current: fc.constantFrom(...steps),
    // macOS에는 셸 단계가 없으므로 셸 확정 여부는 Windows에서만 의미가 있다
    shellConfirmed: steps.includes('shell') ? fc.boolean() : fc.constant(false),
    detecting: fc.boolean(),
    hasInstalled: fc.boolean(),
    installing: fc.boolean(),
    hasInstallation: fc.boolean(),
    selectionReady: fc.boolean(),
    loggedIn: fc.boolean(),
    submitting: fc.boolean(),
  }),
);

/** 진행 중인 작업(감지, 훅 설치, 보관)이 없는 입력 */
const idleInputArb = inputArb.map((input): StepperInput => ({
  ...input,
  detecting: false,
  installing: false,
  submitting: false,
}));

describe('stepsFor', () => {
  it('FR-START-002 시나리오 1: macOS는 감지, 훅 설치, 선택, 로그인 확인의 네 단계다', () => {
    expect(stepsFor(false)).toEqual(MAC);
  });

  it('FR-START-002 시나리오 7: Windows는 셸 선택을 맨 앞에 둔 다섯 단계다', () => {
    expect(stepsFor(true)).toEqual(WIN);
  });

  it('FR-START-011 시나리오 3: macOS 단계에는 셸 선택이 없다', () => {
    expect(stepsFor(false)).not.toContain('shell');
  });
});

describe('nextStep, prevStep', () => {
  it('FR-START-002: macOS는 감지, 훅 설치, 선택, 로그인 확인 순서로 다음 단계로 간다', () => {
    expect(nextStep(MAC, 'detect')).toBe('install');
    expect(nextStep(MAC, 'install')).toBe('select');
    expect(nextStep(MAC, 'select')).toBe('login');
    expect(nextStep(MAC, 'login')).toBeUndefined();
  });

  it('FR-START-002 시나리오 6: 로그인 확인 단계에서 이전으로 가면 선택 단계다', () => {
    expect(prevStep(MAC, 'login')).toBe('select');
    expect(prevStep(MAC, 'select')).toBe('install');
    expect(prevStep(MAC, 'install')).toBe('detect');
    expect(prevStep(MAC, 'detect')).toBeUndefined();
  });

  it('FR-START-002 시나리오 8: Windows의 셸 선택 단계 다음은 감지 단계다', () => {
    expect(nextStep(WIN, 'shell')).toBe('detect');
    expect(nextStep(WIN, 'detect')).toBe('install');
    expect(nextStep(WIN, 'login')).toBeUndefined();
  });

  it('FR-START-005 시나리오 10: Windows의 감지 단계 이전은 셸 선택 단계이고, 셸 선택 단계가 처음이다', () => {
    expect(prevStep(WIN, 'detect')).toBe('shell');
    expect(prevStep(WIN, 'shell')).toBeUndefined();
  });

  test.prop([stepsArb.chain((steps) => fc.tuple(fc.constant(steps), fc.constantFrom(...steps)))])(
    'FR-START-002: 다음으로 갔다가 이전으로 오면 원래 단계다',
    ([steps, step]) => {
      const next = nextStep(steps, step);
      if (next !== undefined) expect(prevStep(steps, next)).toBe(step);
    },
  );

  test.prop([stepsArb.chain((steps) => fc.tuple(fc.constant(steps), fc.constantFrom(...steps)))])(
    'FR-START-002: 다음 단계와 이전 단계는 단계 목록에서 바로 옆 단계다',
    ([steps, step]) => {
      const index = steps.indexOf(step);

      expect(nextStep(steps, step)).toBe(steps[index + 1]);
      expect(prevStep(steps, step)).toBe(index > 0 ? steps[index - 1] : undefined);
    },
  );
});

describe('indicators', () => {
  it('FR-START-002 시나리오 1: 창이 열리면 네 단계가 순서대로 나오고 감지 단계가 지금 단계다', () => {
    const got = indicators(
      at('detect', { detecting: true, hasInstalled: false, hasInstallation: false }),
    );

    expect(got.map((indicator) => indicator.id)).toEqual(['detect', 'install', 'select', 'login']);
    expect(got.map((indicator) => indicator.current)).toEqual([true, false, false, false]);
  });

  it('FR-START-002 시나리오 7: Windows에서 창이 열리면 다섯 단계가 순서대로 나오고 셸 선택 단계가 지금 단계다', () => {
    const got = indicators(
      winAt('shell', {
        shellConfirmed: false,
        hasInstalled: false,
        hasInstallation: false,
      }),
    );

    expect(got.map((indicator) => indicator.id)).toEqual([
      'shell',
      'detect',
      'install',
      'select',
      'login',
    ]);
    expect(got.map((indicator) => indicator.current)).toEqual([true, false, false, false, false]);
  });

  it('FR-START-002 시나리오 7: Windows에서 셸을 확정하기 전에는 셸 선택이 진행 중이고 나머지 단계는 시작 전이다', () => {
    // 감지는 셸 선택 단계에서 다음을 눌러야 시작한다(FR-START-013)
    const input = winAt('shell', {
      shellConfirmed: false,
      hasInstalled: false,
      hasInstallation: false,
    });

    expect(indicators(input).map((indicator) => indicator.status)).toEqual([
      'running',
      'pending',
      'pending',
      'pending',
      'pending',
    ]);
  });

  it('FR-START-002 시나리오 8: Windows 셸 선택 단계에서 다음을 누르면 셸 선택은 끝남, 감지 단계는 지금 단계다', () => {
    const input = winAt('detect', {
      detecting: true,
      hasInstalled: false,
      hasInstallation: false,
    });

    expect(statusOf(input, 'shell')?.status).toBe('done');
    expect(statusOf(input, 'shell')?.current).toBe(false);
    expect(statusOf(input, 'detect')?.current).toBe(true);
    expect(statusOf(input, 'detect')?.status).toBe('running');
  });

  it('FR-START-011 시나리오 3: macOS 단계 표시줄에는 셸 선택 단계가 없다', () => {
    expect(indicators(at('detect')).map((indicator) => indicator.id)).not.toContain('shell');
  });

  it('FR-START-002 시나리오 2: 앱 시작 감지 중에는 훅 설치, 선택, 로그인 확인 단계가 시작 전이다', () => {
    const input = at('detect', { detecting: true, hasInstalled: false, hasInstallation: false });

    expect(statusOf(input, 'detect')?.current).toBe(true);
    expect(statusOf(input, 'detect')?.status).toBe('running');
    expect(statusOf(input, 'install')?.status).toBe('pending');
    expect(statusOf(input, 'select')?.status).toBe('pending');
    expect(statusOf(input, 'login')?.status).toBe('pending');
  });

  it('FR-START-002 시나리오 3: 감지와 설치가 끝나고 선택 단계로 오면 감지와 훅 설치 단계가 끝남이다', () => {
    const input = at('select');

    expect(statusOf(input, 'detect')?.status).toBe('done');
    expect(statusOf(input, 'install')?.status).toBe('done');
    expect(statusOf(input, 'select')?.current).toBe(true);
  });

  it('FR-START-002 시나리오 4: 감지 단계에 머문 채 감지가 끝나면 감지 단계는 끝남이고 지금 단계로 남는다', () => {
    const input = at('detect', { hasInstallation: false, installing: true });

    expect(statusOf(input, 'detect')?.status).toBe('done');
    expect(statusOf(input, 'detect')?.current).toBe(true);
  });

  it('FR-START-002 시나리오 5: 훅 설치 단계로 넘어가면 감지 단계는 끝남, 훅 설치 단계는 지금 단계다', () => {
    const input = at('install');

    expect(statusOf(input, 'detect')?.status).toBe('done');
    expect(statusOf(input, 'detect')?.current).toBe(false);
    expect(statusOf(input, 'install')?.current).toBe(true);
  });

  it('FR-START-002: 훅 설치가 진행 중이면 훅 설치 단계는 진행 중이다', () => {
    const input = at('install', { installing: true });

    expect(statusOf(input, 'install')?.status).toBe('running');
  });

  it('FR-START-002: 감지 단계에 있는 동안 감지 뒤 훅 설치가 진행 중이면 훅 설치 단계도 진행 중이다', () => {
    const input = at('detect', { installing: true, hasInstallation: false });

    expect(statusOf(input, 'install')?.status).toBe('running');
    expect(statusOf(input, 'install')?.current).toBe(false);
  });

  it('FR-START-002: 선택 단계에서 아직 고르지 않았으면 선택 단계는 진행 중, 로그인 확인 단계는 시작 전이다', () => {
    const input = at('select', { selectionReady: false });

    expect(statusOf(input, 'select')?.status).toBe('running');
    expect(statusOf(input, 'login')?.status).toBe('pending');
  });

  it('FR-START-002: 에이전트와 모델을 모두 고르면 선택 단계는 끝남이다', () => {
    const input = at('select', { selectionReady: true });

    expect(statusOf(input, 'select')?.status).toBe('done');
    expect(statusOf(input, 'select')?.current).toBe(true);
  });

  it('FR-START-002: 로그인 확인 단계에서 로그인됨이 아니면 진행 중이고, 로그인됨이면 끝남이다', () => {
    expect(statusOf(at('login', { selectionReady: true }), 'login')?.status).toBe('running');
    expect(statusOf(at('login', { selectionReady: true, loggedIn: true }), 'login')?.status).toBe(
      'done',
    );
  });

  it('FR-START-002: Windows에서 셸을 확정한 뒤 셸 선택 단계로 돌아와도 셸 선택 단계는 끝남이다', () => {
    const input = winAt('shell');

    expect(statusOf(input, 'shell')?.status).toBe('done');
    expect(statusOf(input, 'shell')?.current).toBe(true);
  });

  test.prop([inputArb])(
    'FR-START-002: 단계 표시줄에는 그 OS의 단계가 순서대로 나오고 지금 단계는 하나뿐이다',
    (input) => {
      const got = indicators(input);

      expect(got.map((indicator) => indicator.id)).toEqual([...input.steps]);
      expect(got.filter((indicator) => indicator.current).map((indicator) => indicator.id)).toEqual(
        [input.current],
      );
    },
  );

  test.prop([inputArb])(
    'FR-START-002 시나리오 2: 감지 중이면 감지 뒤의 단계는 모두 시작 전이다',
    (input) => {
      // 앱 시작 감지 중: 아직 설치 결과도, 선택도, 로그인 확인 결과도 없다
      const detecting: StepperInput = {
        ...input,
        current: 'detect',
        shellConfirmed: input.steps.includes('shell'),
        detecting: true,
        installing: false,
        hasInstallation: false,
        selectionReady: false,
        loggedIn: false,
      };
      const got = indicators(detecting);
      const afterDetect = got.slice(got.findIndex((indicator) => indicator.id === 'detect') + 1);

      expect(afterDetect.map((indicator) => indicator.status)).toEqual([
        'pending',
        'pending',
        'pending',
      ]);
    },
  );

  test.prop([inputArb])('FR-START-002: 셸 선택 단계는 셸을 확정했을 때만 끝남이다', (input) => {
    const shell = statusOf(input, 'shell');
    if (shell === undefined) return;

    expect(shell.status === 'done').toBe(input.shellConfirmed);
  });

  /** TQ2 답: 각 단계의 일이 끝났는가. 셸 선택은 셸을 확정했을 때 끝난다 */
  const doneOf = (input: StepperInput, id: StepId): boolean => {
    switch (id) {
      case 'shell':
        return input.shellConfirmed;
      case 'detect':
        return !input.detecting;
      case 'install':
        return !input.installing && input.hasInstallation;
      case 'select':
        return input.selectionReady;
      case 'login':
        return input.loggedIn;
    }
  };

  /** TQ2 답: 그 단계의 일(감지, 설치)이 진행 중인가 */
  const workingOf = (input: StepperInput, id: StepId): boolean =>
    (id === 'detect' && input.detecting) || (id === 'install' && input.installing);

  /** 셸을 확정하기 전의 Windows 감지 단계는 아직 시작하지 않았으므로 아래 두 속성에서 뺀다 */
  const detectionStarted = (input: StepperInput, id: StepId): boolean =>
    id !== 'detect' || !input.steps.includes('shell') || input.shellConfirmed;

  test.prop([inputArb])('FR-START-002: 일이 끝난 단계는 끝남이다', (input) => {
    for (const indicator of indicators(input)) {
      if (!detectionStarted(input, indicator.id)) continue;
      expect(indicator.status === 'done').toBe(doneOf(input, indicator.id));
    }
  });

  test.prop([inputArb])(
    'FR-START-002: 끝나지 않은 단계는 지금 단계이거나 일이 진행 중이면 진행 중, 그 밖은 시작 전이다',
    (input) => {
      for (const indicator of indicators(input)) {
        if (!detectionStarted(input, indicator.id)) continue;
        if (doneOf(input, indicator.id)) continue;
        expect(indicator.status).toBe(
          indicator.id === input.current || workingOf(input, indicator.id) ? 'running' : 'pending',
        );
      }
    },
  );
});

describe('navigation', () => {
  it('FR-START-005 시나리오 1: 감지 단계에서 감지 중이면 다음과 이전 버튼이 모두 비활성화된다', () => {
    expect(navigation(at('detect', { detecting: true }))).toEqual({
      canPrev: false,
      canNext: false,
    });
    expect(navigation(winAt('detect', { detecting: true }))).toEqual({
      canPrev: false,
      canNext: false,
    });
  });

  it('FR-START-005 시나리오 2: 감지가 끝났고 설치됨인 항목이 없으면 다음 버튼은 비활성화된다', () => {
    expect(navigation(at('detect', { hasInstalled: false })).canNext).toBe(false);
  });

  it('FR-START-005: macOS에서 감지가 끝났고 설치됨인 항목이 있으면 감지 단계의 다음 버튼만 활성화된다', () => {
    expect(navigation(at('detect'))).toEqual({ canPrev: false, canNext: true });
  });

  it('FR-START-005 시나리오 3: 훅 설치 단계에서 설치가 끝나지 않았으면 두 버튼 모두 비활성화되고, 끝나면 둘 다 활성화된다', () => {
    expect(navigation(at('install', { installing: true, hasInstallation: false }))).toEqual({
      canPrev: false,
      canNext: false,
    });
    expect(navigation(at('install'))).toEqual({ canPrev: true, canNext: true });
  });

  it('FR-START-005 시나리오 4: 선택 단계에서 에이전트와 모델이 모두 골라지면 다음 버튼이 활성화된다', () => {
    expect(navigation(at('select', { selectionReady: true }))).toEqual({
      canPrev: true,
      canNext: true,
    });
  });

  it('FR-START-005 시나리오 5: 선택 단계에서 모델이 골라지지 않으면 다음 버튼은 비활성화된다', () => {
    expect(navigation(at('select', { selectionReady: false })).canNext).toBe(false);
  });

  it('FR-START-005 시나리오 6: 로그인 확인 결과가 로그인됨이 아니면 다음 버튼은 비활성화되고 이전 버튼은 활성화된다', () => {
    expect(navigation(at('login', { selectionReady: true, loggedIn: false }))).toEqual({
      canPrev: true,
      canNext: false,
    });
  });

  it('FR-START-005 시나리오 7: 로그인 확인 결과가 로그인됨이면 다음 버튼이 활성화된다', () => {
    expect(navigation(at('login', { selectionReady: true, loggedIn: true }))).toEqual({
      canPrev: true,
      canNext: true,
    });
  });

  it('FR-START-005: 보관과 작은 창 전환이 진행 중이면 두 버튼 모두 비활성화된다', () => {
    expect(
      navigation(at('login', { selectionReady: true, loggedIn: true, submitting: true })),
    ).toEqual({ canPrev: false, canNext: false });
  });

  it('FR-START-005 시나리오 8: 고른 에이전트의 훅 설치가 실패했어도 선택 단계의 다음 버튼은 골라졌으면 활성화된다', () => {
    // 설치 실패도 설치가 끝난 결과다
    expect(navigation(at('select', { selectionReady: true, hasInstallation: true })).canNext).toBe(
      true,
    );
  });

  it('FR-START-005 시나리오 9: Windows 셸 선택 단계에서 다음 버튼은 활성화되고 이전 버튼은 비활성화된다', () => {
    expect(
      navigation(
        winAt('shell', { shellConfirmed: false, hasInstalled: false, hasInstallation: false }),
      ),
    ).toEqual({ canPrev: false, canNext: true });
  });

  it('FR-START-005 시나리오 10: Windows 감지 단계에서 감지가 진행 중이 아니면 이전 버튼이 활성화된다', () => {
    expect(navigation(winAt('detect')).canPrev).toBe(true);
    expect(navigation(winAt('detect', { hasInstalled: false })).canPrev).toBe(true);
  });

  test.prop([inputArb])(
    'FR-START-005: 훅 설치가 진행 중이면 어느 단계에서든 두 버튼 모두 비활성화된다',
    (input) => {
      expect(navigation({ ...input, installing: true })).toEqual({
        canPrev: false,
        canNext: false,
      });
    },
  );

  test.prop([inputArb])(
    'FR-START-005: 첫 단계(macOS는 감지, Windows는 셸 선택)에서는 이전 버튼이 늘 비활성화된다',
    (input) => {
      const first = input.steps[0];
      if (first === undefined) return;

      expect(navigation({ ...input, current: first }).canPrev).toBe(false);
    },
  );

  test.prop([idleInputArb])(
    'FR-START-005: 진행 중인 작업이 없으면 첫 단계가 아닌 단계에서는 이전 버튼이 활성화된다',
    (input) => {
      expect(navigation(input).canPrev).toBe(input.current !== input.steps[0]);
    },
  );

  test.prop([idleInputArb])(
    'FR-START-005: 진행 중인 작업이 없으면 셸 선택 단계의 다음 버튼은 늘 활성화된다',
    (input) => {
      expect(navigation({ ...input, steps: WIN, current: 'shell' }).canNext).toBe(true);
    },
  );

  test.prop([inputArb])(
    'FR-START-005: 감지 단계의 다음 버튼은 감지 중이 아니고 설치됨 항목이 있을 때만 활성화된다',
    (input) => {
      const got = navigation({ ...input, current: 'detect' });

      if (got.canNext) {
        expect(input.detecting).toBe(false);
        expect(input.hasInstalled).toBe(true);
      }
    },
  );

  test.prop([inputArb])(
    'FR-START-005: 선택 단계의 다음 버튼은 에이전트와 모델이 모두 골라졌을 때만 활성화된다',
    (input) => {
      const got = navigation({ ...input, current: 'select' });

      if (got.canNext) expect(input.selectionReady).toBe(true);
    },
  );

  test.prop([inputArb])(
    'FR-START-005: 로그인 확인 단계의 다음 버튼은 로그인됨이고 보관 중이 아닐 때만 활성화된다',
    (input) => {
      const got = navigation({ ...input, current: 'login' });

      if (got.canNext) {
        expect(input.loggedIn).toBe(true);
        expect(input.submitting).toBe(false);
      }
    },
  );

  test.prop([inputArb])(
    'FR-START-005: 로그인 확인 단계에서 보관 중이면 두 버튼 모두 비활성화된다',
    (input) => {
      expect(navigation({ ...input, current: 'login', submitting: true })).toEqual({
        canPrev: false,
        canNext: false,
      });
    },
  );
});
