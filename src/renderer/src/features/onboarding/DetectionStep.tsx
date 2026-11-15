import type { Detection } from '@renderer/types/bindings/Detection';
import StepSection from './StepSection';
import { toRows } from './domain/detectionView';
import type { StepStatus } from './domain/steps';

type Props = {
  readonly status: StepStatus;
  readonly result?: Detection;
  readonly detecting: boolean;
  readonly canRedetect: boolean;
  readonly onRedetect: () => void;
};

/** 감지 단계. 감지 결과를 에이전트와 실행 환경마다 한 줄씩 보여 주고 다시 감지 버튼을 둔다. */
function DetectionStep({
  status,
  result,
  detecting,
  canRedetect,
  onRedetect,
}: Props): React.JSX.Element {
  return (
    <StepSection
      title="1. 설치된 에이전트 감지"
      status={status}
      action={
        <button type="button" disabled={!canRedetect} onClick={onRedetect}>
          다시 감지
        </button>
      }
    >
      {detecting && <p className="onboarding-status">확인 중…</p>}
      {result && (
        <ul>
          {toRows(result).map((row) => (
            <li key={row.key}>
              <span>{row.agentLabel}</span>
              <span>{row.environmentLabel}</span>
              <span>{row.statusLabel}</span>
              {row.detail && <span className="onboarding-detail">{row.detail}</span>}
            </li>
          ))}
        </ul>
      )}
    </StepSection>
  );
}

export default DetectionStep;
