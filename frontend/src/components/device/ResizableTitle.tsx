import { useState, useEffect, useRef } from 'react';

interface ResizableTitleProps extends React.HTMLAttributes<HTMLElement> {
  onResize: (width: number) => void;
  width?: number;
}

/** 可拖拽调整宽度的表头单元格 */
export default function ResizableTitle(props: ResizableTitleProps) {
  const { onResize, width, style, ...restProps } = props;
  const [resizing, setResizing] = useState(false);
  const startX = useRef(0);
  const startW = useRef(0);

  const onMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setResizing(true);
    startX.current = e.clientX;
    startW.current = width || 100;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  };

  useEffect(() => {
    if (!resizing) return;
    const onMouseMove = (e: MouseEvent) => {
      const diff = e.clientX - startX.current;
      onResize(Math.max(50, startW.current + diff));
    };
    const onMouseUp = () => {
      setResizing(false);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);
    return () => {
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
    };
  }, [resizing, onResize]);

  return (
    <th {...restProps} style={{ ...style, position: 'relative' }}>
      {restProps.children}
      <div className="column-resize-handle" onMouseDown={onMouseDown} />
    </th>
  );
}
