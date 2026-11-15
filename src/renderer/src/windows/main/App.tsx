import { lazy, Suspense } from 'react';
import WindowDragArea from '@renderer/features/window-drag/WindowDragArea';

// three가 커서 첫 화면 번들과 분리한다
const TestScene = lazy(() => import('@renderer/scene/TestScene'));

/** 작은 창. 끌어 옮길 수 있는 투명한 영역 위에 시험용 3D를 둔다. */
function App(): React.JSX.Element {
  return (
    <WindowDragArea>
      <Suspense fallback={<div className="test-scene-loading">불러오는 중…</div>}>
        <TestScene />
      </Suspense>
    </WindowDragArea>
  );
}

export default App;
