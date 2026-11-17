/** 단계 하나의 진행 상태. */
export type StepStatus = 'pending' | 'running' | 'done';

/** 시작 설정의 단계. */
export type StepId = 'detect' | 'install' | 'select' | 'login';

/** 단계 순서. 감지, 훅 설치, 에이전트와 모델 선택, 로그인 확인. */
export const STEPS: readonly StepId[] = ['detect', 'install', 'select', 'login'];

/** 이전/다음 버튼과 단계 표시줄을 정하는 입력. */
export type StepperInput = {
  readonly current: StepId;
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
 * 단계의 일이 끝났는가. 감지는 감지 중이 아님, 훅 설치는 설치 중이 아니고 결과가 있음,
 * 선택은 에이전트와 모델이 골라짐, 로그인 확인은 로그인됨이다.
 */
function isDone(input: StepperInput, id: StepId): boolean {
  switch (id) {
    case 'detect':
      return !input.detecting;
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
  const { current, detecting, installing, submitting } = input;
  // 감지, 훅 설치, 보관이 진행 중이면 어느 쪽으로도 옮기지 않는다. 감지 단계는 설치됨 항목이 있어야 넘어간다
  if (detecting || installing || submitting) return { canPrev: false, canNext: false };
  return {
    canPrev: current !== 'detect',
    canNext: isDone(input, current) && (current !== 'detect' || input.hasInstalled),
  };
}

/** 단계 표시줄의 한 칸. */
export type StepIndicator = {
  readonly id: StepId;
  /** 지금 보고 있는 단계 */
  readonly current: boolean;
  readonly status: StepStatus;
};

/** 단계 표시줄. STEPS 순서로 각 단계가 지금 단계인지와 진행 상태를 준다. FR-START-002 */
export function indicators(input: StepperInput): readonly StepIndicator[] {
  return STEPS.map((id) => ({
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
export function nextStep(current: StepId): StepId | undefined {
  return STEPS[STEPS.indexOf(current) + 1];
}

/** 이전 단계. 첫 단계면 undefined. */
export function prevStep(current: StepId): StepId | undefined {
  const index = STEPS.indexOf(current);
  return index > 0 ? STEPS[index - 1] : undefined;
}
