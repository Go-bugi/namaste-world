import { useCallback, useEffect, useReducer, useRef, useState } from 'react';
import { api } from '@renderer/api';
import type { Detection } from '@renderer/types/bindings/Detection';
import type { HookInstallation } from '@renderer/types/bindings/HookInstallation';
import DetectionStep from './DetectionStep';
import InstallStep from './InstallStep';
import SelectStep from './SelectStep';
import { canRedetect, initialPanelState, panelReducer } from './domain/detectionView';
import { toInstallRows } from './domain/installView';
import {
  canProceed,
  choicesToQuery,
  initialSelectionState,
  selectionReducer,
  toSelection,
  type AgentChoice,
} from './domain/selection';
import { stepStatuses } from './domain/steps';

/** IPC가 실패했을 때 모델 목록 조회를 다시 해 보는 횟수. Rust 명령 자체는 실패하지 않고 고정 목록을 준다. */
const MODEL_QUERY_RETRIES = 1;

/** 시작 설정 창. 감지, 훅 설치, 에이전트와 모델 선택을 차례로 보여 주고 다음 버튼으로 작은 창으로 넘어간다. */
function Onboarding(): React.JSX.Element {
  const [panel, dispatchPanel] = useReducer(panelReducer, initialPanelState);
  const [selection, dispatchSelection] = useReducer(selectionReducer, initialSelectionState);
  const [installation, setInstallation] = useState<HookInstallation>();
  // 끝나기를 기다리는 훅 설치 수. 앱 시작 설치 하나에서 시작해 다시 감지할 때마다 하나씩 는다
  const [pendingInstalls, setPendingInstalls] = useState(1);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string>();
  // 감지 결과를 받을 때마다 늘린다. 늘어나면 그 결과로 모델 목록을 조회한다
  const [detectionCount, setDetectionCount] = useState(0);

  // 마지막으로 모델 목록을 조회한 감지 결과 번호. 감지 결과 하나에 한 번만 조회한다
  const queriedDetection = useRef(0);
  // 가장 최근 감지 결과 번호와, 항목마다 목록을 조회한 감지 결과 번호
  const latestDetection = useRef(0);
  const queriedAt = useRef<Record<string, number>>({});
  // 항목마다 마지막으로 보낸 조회 번호. 늦게 온 이전 조회 결과를 버린다
  const requestIds = useRef<Record<string, number>>({});
  // 앱 시작 설치의 끝을 무엇으로 셌는지. 같은 설치를 이벤트와 응답으로 두 번 세지 않는다
  const startupInstall = useRef<'pending' | 'byResponse' | 'done'>('pending');

  const queryModels = useCallback((choice: AgentChoice): void => {
    const id = (requestIds.current[choice.key] ?? 0) + 1;
    requestIds.current[choice.key] = id;
    queriedAt.current[choice.key] = latestDetection.current;
    dispatchSelection({ type: 'modelsLoading', key: choice.key });
    const attempt = (retries: number): void => {
      void api.getAgentModels(choice.agent, choice.environment).match(
        (list) => {
          if (requestIds.current[choice.key] === id) {
            dispatchSelection({ type: 'modelsLoaded', key: choice.key, list });
          }
        },
        (e) => {
          if (retries > 0 && requestIds.current[choice.key] === id) attempt(retries - 1);
          else console.error('모델 목록 조회 실패', e);
        },
      );
    };
    attempt(MODEL_QUERY_RETRIES);
  }, []);

  const onDetected = useCallback((result: Detection): void => {
    latestDetection.current += 1;
    dispatchPanel({ type: 'detected', result });
    dispatchSelection({ type: 'detected', detection: result });
    setDetectionCount((count) => count + 1);
  }, []);

  useEffect(() => {
    // StrictMode가 effect를 두 번 돌려도 먼저 정리된 쪽의 비동기 결과는 쓰지 않는다
    let cancelled = false;
    const unlisteners: Array<() => void> = [];
    // 구독이 끝나기 전에 정리되면 받은 해제 함수를 바로 부른다
    const keep = (unlisten: () => void): void => {
      if (cancelled) unlisten();
      else unlisteners.push(unlisten);
    };
    const installFinished = (): void => setPendingInstalls((count) => Math.max(0, count - 1));

    void api
      .onHookInstallation((result) => {
        if (cancelled) return;
        setInstallation(result);
        // 응답으로 이미 센 앱 시작 설치의 이벤트면 다시 세지 않는다
        if (startupInstall.current === 'byResponse') {
          startupInstall.current = 'done';
          return;
        }
        startupInstall.current = 'done';
        installFinished();
      })
      .map(keep);
    // 이벤트를 놓쳤을 때를 위해 보관된 결과도 읽는다. 이벤트로 먼저 받은 새 결과는 덮지 않는다
    void api.getHookInstallation().map((result) => {
      if (cancelled) return;
      setInstallation((current) => current ?? result);
      if (startupInstall.current === 'pending') {
        startupInstall.current = 'byResponse';
        installFinished();
      }
    });

    // 저장한 선택을 먼저 넣어야 감지 결과가 왔을 때 미리 고를 수 있다
    void api
      .getSavedSelection()
      .match(
        (saved) => {
          if (!cancelled) dispatchSelection({ type: 'savedLoaded', saved });
        },
        () => {
          if (!cancelled) dispatchSelection({ type: 'savedLoaded', saved: undefined });
        },
      )
      .then(() =>
        api.getAgentDetection().match(
          (result) => {
            if (!cancelled) onDetected(result);
          },
          () => {
            if (!cancelled) dispatchPanel({ type: 'failed' });
          },
        ),
      );

    return () => {
      cancelled = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [onDetected]);

  // 감지 결과가 선택 상태에 반영된 뒤에 조회할 항목을 고른다
  const { choices, selectedKey } = selection;
  useEffect(() => {
    if (detectionCount === 0 || queriedDetection.current === detectionCount) return;
    queriedDetection.current = detectionCount;
    choicesToQuery(choices, selectedKey).forEach(queryModels);
  }, [detectionCount, choices, selectedKey, queryModels]);

  const redetect = (): void => {
    dispatchPanel({ type: 'redetectStarted' });
    // 다시 감지가 끝나면 Rust가 훅 설치를 한 번 더 돌린다
    setPendingInstalls((count) => count + 1);
    void api.redetectAgents().match(onDetected, () => dispatchPanel({ type: 'failed' }));
  };

  const pickAgent = (key: string): void => {
    dispatchSelection({ type: 'agentPicked', key });
    const choice = selection.choices.find((candidate) => candidate.key === key);
    if (choice === undefined) return;
    // WSL 항목은 고를 때 조회하므로, 이번 감지 결과로 아직 조회하지 않았으면 새로 조회한다
    const stale =
      choice.environment.kind === 'wsl' && queriedAt.current[key] !== latestDetection.current;
    if (selection.models[key] === undefined || stale) queryModels(choice);
  };

  const next = (): void => {
    const picked = toSelection(selection);
    if (picked === undefined) return;
    setSubmitting(true);
    setError(undefined);
    // 성공하면 이 창이 닫히므로 따로 할 일이 없다
    void api.finishOnboarding(picked).mapErr((e) => {
      setSubmitting(false);
      setError(`작은 창을 열지 못했습니다: ${e.message}`);
    });
  };

  const installing = pendingInstalls > 0;
  const steps = stepStatuses({
    detecting: panel.detecting,
    hasDetection: panel.result !== undefined,
    installing,
    hasInstallation: installation !== undefined,
  });
  const installFailure =
    installation === undefined
      ? undefined
      : toInstallRows(installation).find(
          (row) => row.key === selection.selectedKey && row.statusLabel === '실패',
        );

  return (
    <main className="onboarding">
      <h1>시작 설정</h1>
      <DetectionStep
        status={steps.detect}
        detecting={panel.detecting}
        canRedetect={canRedetect(panel)}
        onRedetect={redetect}
        {...(panel.result === undefined ? {} : { result: panel.result })}
      />
      <InstallStep
        status={steps.install}
        installing={installing}
        {...(installation === undefined ? {} : { installation })}
      />
      <SelectStep
        status={steps.select}
        state={selection}
        canProceed={canProceed(selection, panel.detecting)}
        submitting={submitting}
        onPickAgent={pickAgent}
        onPickModel={(value) => dispatchSelection({ type: 'modelPicked', value })}
        onNext={next}
        {...(installFailure === undefined ? {} : { installFailure })}
        {...(error === undefined ? {} : { error })}
      />
    </main>
  );
}

export default Onboarding;
