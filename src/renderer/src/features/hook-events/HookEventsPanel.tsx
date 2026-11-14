import { useEffect, useState } from 'react';
import { api } from '@renderer/api';
import type { HookEvent } from '@renderer/types/bindings/HookEvent';
import type { HookInstallation } from '@renderer/types/bindings/HookInstallation';
import { appendEvent, toEventRow, toInstallRows } from './hookEventsView';

/** 훅 설치 결과와, 이번 실행에서 받은 훅 이벤트를 받은 순서대로 보여 준다. */
function HookEventsPanel(): React.JSX.Element {
  const [installation, setInstallation] = useState<HookInstallation>();
  const [events, setEvents] = useState<readonly HookEvent[]>([]);

  useEffect(() => {
    let active = true;
    const unlisteners: Array<() => void> = [];
    // 구독이 끝나기 전에 언마운트되면 받은 해제 함수를 바로 부른다
    const keep = (unlisten: () => void): void => {
      if (active) unlisteners.push(unlisten);
      else unlisten();
    };

    void api.onHookEvent((event) => setEvents((list) => appendEvent(list, event))).map(keep);
    void api.onHookInstallation((result) => setInstallation(result)).map(keep);
    // 이벤트로 먼저 받은 새 결과를 늦게 온 응답이 덮지 않게, 아직 결과가 없을 때만 쓴다
    void api.getHookInstallation().map((result) => {
      if (active) setInstallation((current) => current ?? result);
    });

    return () => {
      active = false;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []);

  return (
    <section className="hook-events">
      <header>
        <h2>훅 이벤트</h2>
        {installation === undefined && <span className="hook-events-muted">설치 확인 중…</span>}
      </header>
      {installation && (
        <ul>
          {toInstallRows(installation).map((row) => (
            <li key={row.key}>
              <span>{row.agentLabel}</span>
              <span>{row.environmentLabel}</span>
              <span>{row.statusLabel}</span>
              {row.detail && <span className="hook-events-detail">{row.detail}</span>}
            </li>
          ))}
        </ul>
      )}
      {events.length === 0 ? (
        <p className="hook-events-muted">아직 받은 이벤트가 없습니다</p>
      ) : (
        <ol>
          {events.map((event, index) => {
            const row = toEventRow(event, index);
            return (
              <li key={row.key}>
                <span className="hook-events-detail">{row.time}</span>
                <span>{row.agentLabel}</span>
                <span>{row.event}</span>
              </li>
            );
          })}
        </ol>
      )}
    </section>
  );
}

export default HookEventsPanel;
