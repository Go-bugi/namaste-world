import { useEffect, useState } from 'react';
import { api } from '@renderer/api';

function Versions(): React.JSX.Element {
  const [appVersion, setAppVersion] = useState<string>();
  const [tauriVersion, setTauriVersion] = useState<string>();

  useEffect(() => {
    void api.getAppVersion().then(setAppVersion);
    void api.getTauriVersion().then(setTauriVersion);
  }, []);

  return (
    <ul className="versions">
      {appVersion && <li className="app-version">Namaste World v{appVersion}</li>}
      {tauriVersion && <li className="tauri-version">Tauri v{tauriVersion}</li>}
    </ul>
  );
}

export default Versions;
