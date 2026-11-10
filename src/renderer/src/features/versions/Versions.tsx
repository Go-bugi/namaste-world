import { useEffect, useState } from 'react';
import { ResultAsync } from 'neverthrow';
import { api } from '@renderer/api';

type VersionInfo = { readonly app: string; readonly tauri: string };

function Versions(): React.JSX.Element | null {
  const [versions, setVersions] = useState<VersionInfo>();

  useEffect(() => {
    let ignore = false;
    void ResultAsync.combine([api.getAppVersion(), api.getTauriVersion()]).match(
      ([app, tauri]) => {
        if (!ignore) setVersions({ app, tauri });
      },
      // 버전을 못 읽어도 화면에는 영향이 없으므로 목록을 그리지 않는다
      () => undefined,
    );
    return () => {
      ignore = true; // 늦게 온 결과를 버린다
    };
  }, []);

  if (!versions) return null;
  return (
    <ul className="versions">
      <li className="app-version">Namaste World v{versions.app}</li>
      <li className="tauri-version">Tauri v{versions.tauri}</li>
    </ul>
  );
}

export default Versions;
