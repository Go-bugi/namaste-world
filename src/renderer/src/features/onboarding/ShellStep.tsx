import type { ShellKind } from '@renderer/types/bindings/ShellKind';
import StepSection from './StepSection';
import { shellDescription, shellLabel } from './domain/shell';

type Props = {
  readonly options: readonly ShellKind[];
  readonly selected: ShellKind;
  readonly onPick: (shell: ShellKind) => void;
};

/** 셸 선택 단계(Windows). 에이전트를 실행할 셸을 고른다. 다음을 누르면 고른 셸로 감지한다. */
function ShellStep({ options, selected, onPick }: Props): React.JSX.Element {
  return (
    <StepSection title="에이전트를 실행할 셸">
      <p className="onboarding-status">터미널에서 에이전트를 쓸 때와 같은 셸을 고르세요.</p>
      <ul className="onboarding-fields" role="radiogroup" aria-label="셸">
        {options.map((shell) => (
          <li key={shell}>
            <label>
              <input
                type="radio"
                name="shell"
                value={shell}
                checked={shell === selected}
                onChange={() => onPick(shell)}
              />
              {shellLabel(shell)}
            </label>
            <span className="onboarding-detail">{shellDescription(shell)}</span>
          </li>
        ))}
      </ul>
    </StepSection>
  );
}

export default ShellStep;
