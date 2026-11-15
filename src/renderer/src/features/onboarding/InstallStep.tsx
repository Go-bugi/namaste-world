import type { HookInstallation } from '@renderer/types/bindings/HookInstallation';
import StepSection from './StepSection';
import { toInstallRows } from './domain/installView';
import type { StepStatus } from './domain/steps';

type Props = {
  readonly status: StepStatus;
  readonly installation?: HookInstallation;
  readonly installing: boolean;
};

/** 훅 설치 단계. 설치 결과를 에이전트와 실행 환경마다 보여 주고, 실패면 이유를 함께 보여 준다. */
function InstallStep({ status, installation, installing }: Props): React.JSX.Element {
  return (
    <StepSection title="2. 훅 설치" status={status}>
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
