import { api } from '@renderer/api';

type Props = { readonly children?: React.ReactNode };

/** 안 어디를 누르고 끌어도 창이 따라 움직이는 영역. 테두리 없는 작은 창에서 쓴다. */
function WindowDragArea({ children }: Props): React.JSX.Element {
  const startDrag = (event: React.PointerEvent<HTMLDivElement>): void => {
    // 왼쪽 버튼만 끌기로 본다
    if (event.button !== 0) return;
    void api.startWindowDrag().mapErr((e) => console.error('창 끌기 실패', e));
  };

  return (
    <div className="window-drag-area" onPointerDown={startDrag}>
      {children}
    </div>
  );
}

export default WindowDragArea;
