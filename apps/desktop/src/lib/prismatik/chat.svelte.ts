class ChatState {
  open = $state(false);
  toggle() { this.open = !this.open; }
}

export const chat = new ChatState();
