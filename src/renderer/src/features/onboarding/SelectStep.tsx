import type { InstallRow } from './domain/installView';
import type { SelectionState } from './domain/selection';
import StepSection from './StepSection';

type Props = {
  readonly state: SelectionState;
  /** 고른 에이전트의 훅 설치 실패 줄. 실패가 없으면 undefined */
  readonly installFailure?: InstallRow;
  readonly canReload: boolean;
  readonly onPickAgent: (key: string) => void;
  readonly onPickModel: (value: string) => void;
  readonly onReload: () => void;
};

/** 선택 단계. 설치된 에이전트와 그 모델을 고르고, 고른 에이전트의 모델 목록을 다시 불러올 수 있다. */
function SelectStep({
  state,
  installFailure,
  canReload,
  onPickAgent,
  onPickModel,
  onReload,
}: Props): React.JSX.Element {
  const models = state.selectedKey === undefined ? undefined : state.models[state.selectedKey];
  const options = models?.status === 'ready' ? models.list.options : [];

  return (
    <StepSection
      title="사용할 에이전트와 모델"
      action={
        <button type="button" disabled={!canReload} onClick={onReload}>
          모델 목록 다시 불러오기
        </button>
      }
    >
      {state.choices.length === 0 ? (
        <p className="onboarding-status">
          설치된 에이전트가 없습니다. 설치한 뒤 다시 감지를 눌러 주세요.
        </p>
      ) : (
        <div className="onboarding-fields">
          <label>
            에이전트
            <select
              value={state.selectedKey ?? ''}
              onChange={(event) => onPickAgent(event.target.value)}
            >
              {state.selectedKey === undefined && <option value="">선택하세요</option>}
              {state.choices.map((choice) => (
                <option key={choice.key} value={choice.key}>
                  {choice.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            모델
            {models?.status === 'loading' ? (
              <span className="onboarding-status">불러오는 중…</span>
            ) : (
              <select
                value={state.model ?? ''}
                disabled={options.length === 0}
                onChange={(event) => onPickModel(event.target.value)}
              >
                {state.model === undefined && <option value="">선택하세요</option>}
                {options.map((option) => (
                  <option key={option.value} value={option.value}>
                    {/* 별칭이면 지금 가리키는 실제 모델 ID를 함께 보여 준다 */}
                    {option.detail === undefined
                      ? option.label
                      : `${option.label} · ${option.detail}`}
                  </option>
                ))}
              </select>
            )}
          </label>
          {installFailure && (
            <p className="onboarding-error">
              {installFailure.agentLabel} · {installFailure.environmentLabel} 훅 설치 실패:{' '}
              {installFailure.detail}
            </p>
          )}
        </div>
      )}
    </StepSection>
  );
}

export default SelectStep;
