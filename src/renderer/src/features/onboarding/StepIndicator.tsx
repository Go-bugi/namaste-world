import type { StepId, StepIndicator as Indicator, StepStatus } from './domain/stepper';

const STEP_LABEL: Record<StepId, string> = {
  shell: '셸 선택',
  detect: '감지',
  install: '훅 설치',
  select: '에이전트와 모델',
  login: '로그인 확인',
};

const STATUS_LABEL: Record<StepStatus, string> = {
  pending: '대기',
  running: '진행 중',
  done: '완료',
};

type Props = { readonly items: readonly Indicator[] };

/** 단계 표시줄. 단계를 순서대로 보여 주고 지금 단계와 각 단계의 진행 상태를 표시한다. */
function StepIndicator({ items }: Props): React.JSX.Element {
  return (
    <ol className="onboarding-stepper">
      {items.map((item, index) => (
        <li
          key={item.id}
          data-status={item.status}
          aria-current={item.current ? 'step' : undefined}
        >
          <span>
            {index + 1}. {STEP_LABEL[item.id]}
          </span>
          <span className="onboarding-status">{STATUS_LABEL[item.status]}</span>
        </li>
      ))}
    </ol>
  );
}

export default StepIndicator;
