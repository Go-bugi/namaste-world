import { lazy, Suspense } from 'react';
import Versions from '@renderer/features/versions/Versions';
import AgentDetectionPanel from '@renderer/features/agent-detection/AgentDetectionPanel';
import logo from '@renderer/assets/logo.png';
import { api } from '@renderer/api';

// three가 커서 첫 화면 번들과 분리한다
const DemoScene = lazy(() => import('@renderer/scene/DemoScene'));

function App(): React.JSX.Element {
  const ipcHandle = (): void => {
    void api.ping().match(
      () => undefined,
      (e) => console.error('ping 실패', e),
    );
  };

  return (
    <>
      <Suspense fallback={<img alt="logo" className="logo" src={logo} />}>
        <DemoScene />
      </Suspense>
      <div className="creator">Powered by Tauri + Vite</div>
      <div className="text">
        Build a Tauri app with <span className="react">React</span>
        &nbsp;and <span className="ts">TypeScript</span>
      </div>
      <p className="tip">
        Right-click and choose <code>Inspect</code> to open the devTool
      </p>
      <div className="actions">
        <div className="action">
          <a href="https://v2.tauri.app/start/" target="_blank" rel="noreferrer">
            Documentation
          </a>
        </div>
        <div className="action">
          <a target="_blank" rel="noreferrer" onClick={ipcHandle}>
            Send IPC
          </a>
        </div>
      </div>
      <AgentDetectionPanel />
      <Versions></Versions>
    </>
  );
}

export default App;
