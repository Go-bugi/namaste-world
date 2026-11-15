/** 단계 하나의 진행 상태. */
export type StepStatus = 'pending' | 'running' | 'done';

/** 세 단계의 상태를 정하는 입력. has*는 결과를 한 번이라도 받았는지다. */
export type StepInput = {
  readonly detecting: boolean;
  readonly hasDetection: boolean;
  readonly installing: boolean;
  readonly hasInstallation: boolean;
};

/** 감지, 훅 설치, 선택 단계의 상태. */
export type StepStatuses = {
  readonly detect: StepStatus;
  readonly install: StepStatus;
  readonly select: StepStatus;
};

/**
 * 감지·훅 설치·선택 단계의 상태. detecting이면 감지 running이고 나머지는 pending(다시 감지 중 포함).
 * 감지가 끝났고 installing이면 설치 running, 선택 pending. 둘 다 끝나면 감지·설치 done, 선택 running.
 */
export function stepStatuses({
  detecting,
  hasDetection,
  installing,
  hasInstallation,
}: StepInput): StepStatuses {
  if (detecting || !hasDetection) {
    return { detect: 'running', install: 'pending', select: 'pending' };
  }
  if (installing || !hasInstallation) {
    return { detect: 'done', install: 'running', select: 'pending' };
  }
  return { detect: 'done', install: 'done', select: 'running' };
}
