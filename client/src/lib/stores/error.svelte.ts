type ErrorEntry = { id: number; message: string };

let nextId = 0;

export const errorStore = $state<{ errors: ErrorEntry[] }>({ errors: [] });

export function addError(message: string, duration = 5000) {
  const id = nextId++;
  errorStore.errors.push({ id, message });
  if (duration > 0) setTimeout(() => dismissError(id), duration);
}

export function dismissError(id: number) {
  errorStore.errors = errorStore.errors.filter((e) => e.id !== id);
}
