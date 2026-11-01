import { useLayoutEffect, useRef } from 'react';

export function WorkbenchModal({ labelId, busy, onDismiss, children }: {
  labelId: string; busy: boolean; onDismiss: () => void; children: React.ReactNode;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useLayoutEffect(() => {
    const dialog = ref.current!;
    dialog.showModal();
    return () => dialog.close();
  }, []);
  return <dialog ref={ref} className="workbench-dialog" aria-labelledby={labelId} onKeyDown={event => {
    if (event.key !== 'Tab') return;
    const controls = [...event.currentTarget.querySelectorAll<HTMLElement>('button, input, select, textarea, [tabindex]')]
      .filter(control => !control.matches(':disabled') && control.tabIndex >= 0 && control.getClientRects().length > 0);
    const first = controls[0], last = controls.at(-1);
    if (event.shiftKey && document.activeElement === first || !event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      (event.shiftKey ? last : first)?.focus();
    }
  }} onCancel={event => {
    event.preventDefault();
    if (!busy) onDismiss();
  }}>{children}</dialog>;
}
