import { useState } from 'react';
import type { AgentChoice } from './domain/selection';
import { canRecheck, loginGuide, loginStatusLabel, type LoginState } from './domain/login';
import StepSection from './StepSection';

type Props = {
  readonly choice: AgentChoice;
  readonly state: LoginState;
  readonly onRecheck: () => void;
};

/** 로그인 확인 단계. 고른 에이전트의 로그인 상태를 보여 주고, 로그인 안 됨이면 터미널에서 실행할 명령과 복사 버튼을 보여 준다. */
function LoginStep({ choice, state, onRecheck }: Props): React.JSX.Element {
  const [copied, setCopied] = useState<string>();
  const status = state.kind === 'checked' && state.key === choice.key ? state.status : undefined;
  const guide =
    status === undefined ? undefined : loginGuide(choice.agent, choice.environment, status);

  const copy = (command: string): void => {
    // 클릭 핸들러 안에서 불러야 웹뷰가 클립보드 쓰기를 허용한다. 거부되면 명령 글자를 직접 선택해 복사한다
    void navigator.clipboard.writeText(command).then(
      () => setCopied(command),
      () => setCopied(undefined),
    );
  };

  return (
    <StepSection
      title="로그인 확인"
      action={
        <button type="button" disabled={!canRecheck(state)} onClick={onRecheck}>
          다시 확인
        </button>
      }
    >
      <p>{choice.label}</p>
      {status === undefined ? (
        <p className="onboarding-status">확인 중…</p>
      ) : (
        <p className={status.kind === 'loggedIn' ? 'onboarding-status' : 'onboarding-error'}>
          {loginStatusLabel(status)}
        </p>
      )}
      {guide && (
        <div className="onboarding-login-guide">
          <p>{guide.where}에서 아래 명령을 실행해 로그인한 뒤 다시 확인을 눌러 주세요.</p>
          <div className="onboarding-command">
            <code>{guide.command}</code>
            <button type="button" onClick={() => copy(guide.command)}>
              복사
            </button>
            {copied === guide.command && <span className="onboarding-status">복사했습니다</span>}
          </div>
        </div>
      )}
    </StepSection>
  );
}

export default LoginStep;
