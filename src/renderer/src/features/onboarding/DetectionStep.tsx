import type { Detection } from '@renderer/types/bindings/Detection';
import StepSection from './StepSection';
import { toRows } from './domain/detectionView';

type Props = {
  readonly result?: Detection;
  readonly detecting: boolean;
  readonly canRedetect: boolean;
  readonly onRedetect: () => void;
  /** 셸 환경을 읽지 못했을 때의 안내. 없으면 undefined */
  readonly notice?: string;
};

/**
 * 감지 단계. 감지 결과를 에이전트와 실행 환경마다 한 줄씩 보여 주고 다시 감지 버튼을 둔다.
 * 셸 환경을 읽지 못했으면 그 안내를 함께 보여 준다(FR-AGENT-018).
 */
function DetectionStep({
  result,
  detecting,
  canRedetect,
  onRedetect,
  notice,
}: Props): React.JSX.Element {
  return (
    <StepSection
      title="설치된 에이전트 감지"
      action={
        <button type="button" disabled={!canRedetect} onClick={onRedetect}>
          다시 감지
        </button>
      }
    >
      {detecting && <p className="onboarding-status">확인 중…</p>}
      {!detecting && notice && <p className="onboarding-error">{notice}</p>}
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
