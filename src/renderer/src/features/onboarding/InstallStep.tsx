import type { HookInstallation } from '@renderer/types/bindings/HookInstallation';
import StepSection from './StepSection';
import { toInstallRows } from './domain/installView';

type Props = {
  readonly installation?: HookInstallation;
  readonly installing: boolean;
  readonly onReinstall: () => void;
};

/** 훅 설치 단계. 설치 결과를 에이전트와 실행 환경마다 보여 주고 다시 설치 버튼을 둔다. 설치 중에는 버튼이 꺼진다. */
function InstallStep({ installation, installing, onReinstall }: Props): React.JSX.Element {
  return (
    <StepSection
      title="훅 설치"
      action={
        <button type="button" disabled={installing} onClick={onReinstall}>
          다시 설치
        </button>
      }
    >
      {installing && <p className="onboarding-status">설치 중…</p>}
      {installation && (
        <ul>
          {toInstallRows(installation).map((row) => (
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

export default InstallStep;
