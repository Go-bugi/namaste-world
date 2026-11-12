import { useEffect, useReducer } from 'react';
import { api } from '@renderer/api';
import { canRedetect, initialPanelState, panelReducer, toRows } from './detectionView';

/** 감지한 코딩 에이전트를 환경마다 한 줄씩 보여 주고 다시 감지하는 버튼을 둔다. */
function AgentDetectionPanel(): React.JSX.Element {
  const [state, dispatch] = useReducer(panelReducer, initialPanelState);

  useEffect(() => {
    let ignore = false;
    void api.getAgentDetection().match(
      (result) => {
        if (!ignore) dispatch({ type: 'detected', result });
      },
      () => {
        if (!ignore) dispatch({ type: 'failed' });
      },
    );
    return () => {
      ignore = true; // 늦게 온 결과를 버린다
    };
  }, []);

  const redetect = (): void => {
    dispatch({ type: 'redetectStarted' });
    void api.redetectAgents().match(
      (result) => dispatch({ type: 'detected', result }),
      () => dispatch({ type: 'failed' }),
    );
  };

  return (
    <section className="agent-detection">
      <header>
        <h2>코딩 에이전트</h2>
        {state.detecting && <span className="agent-detection-busy">확인 중…</span>}
        <button type="button" disabled={!canRedetect(state)} onClick={redetect}>
          다시 감지
        </button>
      </header>
      {state.result && (
        <ul>
          {toRows(state.result).map((row) => (
            <li key={row.key}>
              <span>{row.agentLabel}</span>
              <span>{row.environmentLabel}</span>
              <span>{row.statusLabel}</span>
              {row.detail && <span className="agent-detection-detail">{row.detail}</span>}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

export default AgentDetectionPanel;
