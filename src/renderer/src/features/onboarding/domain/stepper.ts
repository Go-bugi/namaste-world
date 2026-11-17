/** 단계 하나의 진행 상태. */
export type StepStatus = 'pending' | 'running' | 'done';

/** 시작 설정의 단계. */
export type StepId = 'shell' | 'detect' | 'install' | 'select' | 'login';

/** 셸 단계가 있으면(Windows) 맨 앞에 'shell'을 둔다. 그 뒤는 감지, 훅 설치, 에이전트와 모델 선택, 로그인 확인. FR-START-002 */
export function stepsFor(hasShellStep: boolean): readonly StepId[] {
  return hasShellStep ? ['shell', ...BASE_STEPS] : BASE_STEPS;
}

/** 모든 OS에 있는 단계. */
const BASE_STEPS: readonly StepId[] = ['detect', 'install', 'select', 'login'];

/** 이전/다음 버튼과 단계 표시줄을 정하는 입력. */
export type StepperInput = {
  /** 이번 OS의 단계 목록(stepsFor) */
  readonly steps: readonly StepId[];
  readonly current: StepId;
  /** 셸 단계에서 다음을 눌러 셸을 확정함 */
  readonly shellConfirmed: boolean;
  /** 앱 시작 감지나 다시 감지가 진행 중 */
  readonly detecting: boolean;
  /** 감지 결과에 설치됨 항목이 하나 이상 */
  readonly hasInstalled: boolean;
  /** 훅 설치(다시 감지 뒤 설치, 다시 설치 포함)가 진행 중 */
  readonly installing: boolean;
  /** 훅 설치 결과를 한 번이라도 받음 */
  readonly hasInstallation: boolean;
  /** 에이전트와 모델이 모두 골라짐 */
  readonly selectionReady: boolean;
  /** 지금 고른 항목의 로그인 확인 결과가 로그인됨 */
  readonly loggedIn: boolean;
  /** 선택 보관과 작은 창 전환이 진행 중 */
  readonly submitting: boolean;
};

/**
 * 감지가 시작됐는가. 셸 단계가 있는 OS(Windows)는 셸을 확정해야 감지가 시작된다. FR-START-013
 */
function detectionStarted(input: StepperInput): boolean {
  return !input.steps.includes('shell') || input.shellConfirmed;
}

/**
 * 단계의 일이 끝났는가. 셸 선택은 셸을 확정함, 감지는 시작됐고 감지 중이 아님,
 * 훅 설치는 설치 중이 아니고 결과가 있음, 선택은 에이전트와 모델이 골라짐, 로그인 확인은 로그인됨이다.
 */
function isDone(input: StepperInput, id: StepId): boolean {
  switch (id) {
    case 'shell':
      return input.shellConfirmed;
    case 'detect':
      return detectionStarted(input) && !input.detecting;
    case 'install':
      return !input.installing && input.hasInstallation;
    case 'select':
      return input.selectionReady;
    case 'login':
      return input.loggedIn;
  }
}

/** 그 단계의 일(감지, 훅 설치)이 진행 중인가. */
function isWorking(input: StepperInput, id: StepId): boolean {
  return (id === 'detect' && input.detecting) || (id === 'install' && input.installing);
}

/** 이전/다음 버튼을 누를 수 있는가. FR-START-005 */
export function navigation(input: StepperInput): {
  readonly canPrev: boolean;
  readonly canNext: boolean;
} {
  const { steps, current, detecting, installing, submitting } = input;
  // 감지, 훅 설치, 보관이 진행 중이면 어느 쪽으로도 옮기지 않는다. 감지 단계는 설치됨 항목이 있어야 넘어간다.
  // 셸 선택 단계는 늘 넘어갈 수 있다. 다음을 누르는 것이 셸을 확정하는 일이기 때문이다
  if (detecting || installing || submitting) return { canPrev: false, canNext: false };
  return {
    canPrev: current !== steps[0],
    canNext:
      current === 'shell' ||
      (isDone(input, current) && (current !== 'detect' || input.hasInstalled)),
  };
}

/** 단계 표시줄의 한 칸. */
export type StepIndicator = {
  readonly id: StepId;
  /** 지금 보고 있는 단계 */
  readonly current: boolean;
  readonly status: StepStatus;
};

/** 단계 표시줄. steps 순서로 각 단계가 지금 단계인지와 진행 상태를 준다. FR-START-002 */
export function indicators(input: StepperInput): readonly StepIndicator[] {
  return input.steps.map((id) => ({
    id,
    current: id === input.current,
    status: isDone(input, id)
      ? 'done'
      : id === input.current || isWorking(input, id)
        ? 'running'
        : 'pending',
  }));
}

/** 다음 단계. 마지막 단계면 undefined. */
export function nextStep(steps: readonly StepId[], current: StepId): StepId | undefined {
  const index = steps.indexOf(current);
  return index < 0 ? undefined : steps[index + 1];
}

/** 이전 단계. 첫 단계면 undefined. */
export function prevStep(steps: readonly StepId[], current: StepId): StepId | undefined {
  const index = steps.indexOf(current);
  return index > 0 ? steps[index - 1] : undefined;
}
