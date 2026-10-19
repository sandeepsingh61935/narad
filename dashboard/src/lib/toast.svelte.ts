export type Toast = {
  id: string;
  message: string;
  onUndo?: () => void;
  isOut?: boolean;
};

class ToastStore {
  items = $state<Toast[]>([]);

  show(message: string, onUndo?: () => void): () => void {
    const id = Math.random().toString(36).slice(2);
    const toast: Toast = { id, message, onUndo, isOut: false };
    this.items = [...this.items, toast];

    const timer = setTimeout(() => {
      this.dismiss(id);
    }, 4200);

    const dismiss = () => {
      clearTimeout(timer);
      this.dismiss(id);
    };

    return dismiss;
  }

  dismiss(id: string) {
    this.items = this.items.map((t) => (t.id === id ? { ...t, isOut: true } : t));
    setTimeout(() => {
      this.items = this.items.filter((t) => t.id !== id);
    }, 180);
  }
}

export const toasts = new ToastStore();

export function toast(message: string, onUndo?: () => void) {
  return toasts.show(message, onUndo);
}
