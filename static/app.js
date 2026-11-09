// Global Chat Client App
(() => {
  // Elements
  const statusDot = document.getElementById('statusDot');
  const statusText = document.getElementById('statusText');
  const userCountEl = document.getElementById('userCount');
  const myNicknameEl = document.getElementById('myNickname');
  const myAvatarEl = document.getElementById('myAvatar');
  const chatContainer = document.getElementById('chatContainer');
  const chatMain = document.getElementById('chatMain');
  const chatForm = document.getElementById('chatForm');
  const messageInput = document.getElementById('messageInput');
  const scrollBottomBtn = document.getElementById('scrollBottomBtn');
  const emojiToggleBtn = document.getElementById('emojiToggleBtn');
  const emojiPicker = document.getElementById('emojiPicker');

  // Modal elements
  const openNicknameModalBtn = document.getElementById('openNicknameModalBtn');
  const nicknameModal = document.getElementById('nicknameModal');
  const closeNicknameModalBtn = document.getElementById('closeNicknameModalBtn');
  const cancelNicknameBtn = document.getElementById('cancelNicknameBtn');
  const saveNicknameBtn = document.getElementById('saveNicknameBtn');
  const nicknameInput = document.getElementById('nicknameInput');

  // State
  let ws = null;
  let currentNickname = localStorage.getItem('rust_chat_nickname') || '';
  let reconnectTimeout = null;
  let reconnectAttempts = 0;
  const maxReconnectDelay = 10000;

  // Avatar colors
  const avatarColors = [
    'linear-gradient(135deg, #6366f1, #8b5cf6)',
    'linear-gradient(135deg, #ec4899, #f43f5e)',
    'linear-gradient(135deg, #06b6d4, #3b82f6)',
    'linear-gradient(135deg, #10b981, #059669)',
    'linear-gradient(135deg, #f59e0b, #d97706)',
    'linear-gradient(135deg, #8b5cf6, #d946ef)',
  ];

  function getAvatarColor(name) {
    if (!name) return avatarColors[0];
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
      hash = name.charCodeAt(i) + ((hash << 5) - hash);
    }
    const index = Math.abs(hash) % avatarColors.length;
    return avatarColors[index];
  }

  function getInitials(name) {
    if (!name) return '?';
    const words = name.trim().split(/\s+/);
    if (words.length >= 2) {
      return (words[0][0] + words[1][0]).toUpperCase();
    }
    return name.slice(0, 2).toUpperCase();
  }

  function updateNicknameUI(name) {
    currentNickname = name;
    localStorage.setItem('rust_chat_nickname', name);
    myNicknameEl.textContent = name;
    myAvatarEl.textContent = getInitials(name);
    myAvatarEl.style.background = getAvatarColor(name);
  }

  function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }

  // WebSocket Connection
  function connectWebSocket() {
    clearTimeout(reconnectTimeout);
    statusDot.className = 'status-dot';
    statusText.textContent = 'Đang kết nối...';

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws`;

    ws = new WebSocket(wsUrl);

    ws.onopen = () => {
      statusDot.className = 'status-dot connected';
      statusText.textContent = 'Đã kết nối';
      reconnectAttempts = 0;

      // Send saved nickname if any
      if (currentNickname) {
        sendWsMessage({
          type: 'set_nickname',
          nickname: currentNickname
        });
      }
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        handleServerMessage(data);
      } catch (err) {
        console.error('Lỗi phân tích tin nhắn từ server:', err);
      }
    };

    ws.onclose = () => {
      statusDot.className = 'status-dot disconnected';
      statusText.textContent = 'Mất kết nối. Đang thử lại...';
      scheduleReconnect();
    };

    ws.onerror = (err) => {
      console.error('Lỗi WebSocket:', err);
      ws.close();
    };
  }

  function scheduleReconnect() {
    reconnectAttempts++;
    const delay = Math.min(1000 * Math.pow(1.5, reconnectAttempts), maxReconnectDelay);
    reconnectTimeout = setTimeout(connectWebSocket, delay);
  }

  function sendWsMessage(payload) {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(payload));
      return true;
    }
    return false;
  }

  // Handle incoming messages
  function handleServerMessage(msg) {
    switch (msg.type) {
      case 'nickname_assigned':
        updateNicknameUI(msg.nickname);
        break;

      case 'user_count':
        userCountEl.textContent = msg.count;
        break;

      case 'history':
        renderHistory(msg.messages);
        break;

      case 'chat':
        appendChatMessage(msg);
        break;

      case 'system':
        appendSystemMessage(msg.content, msg.timestamp);
        break;

      default:
        console.warn('Unknown message type:', msg);
    }
  }

  function renderHistory(messages) {
    // Preserve welcome banner, remove any existing chat items
    const welcome = chatContainer.querySelector('.welcome-banner');
    chatContainer.innerHTML = '';
    if (welcome) {
      chatContainer.appendChild(welcome);
    }

    if (Array.isArray(messages)) {
      messages.forEach(msg => {
        if (msg.type === 'chat') {
          appendChatMessage(msg, false);
        } else if (msg.type === 'system') {
          appendSystemMessage(msg.content, msg.timestamp, false);
        }
      });
    }

    scrollToBottom();
  }

  function appendChatMessage(msg, autoScroll = true) {
    const isMe = msg.nickname === currentNickname;
    const wrapper = document.createElement('div');
    wrapper.className = `message-wrapper ${isMe ? 'me' : 'other'}`;

    const color = getAvatarColor(msg.nickname);
    const initials = getInitials(msg.nickname);

    wrapper.innerHTML = `
      <div class="avatar" style="background: ${color}">${initials}</div>
      <div class="message-content-group">
        <div class="message-header">
          <span class="sender-name">${escapeHtml(msg.nickname)}</span>
          <span class="timestamp">${escapeHtml(msg.timestamp || '')}</span>
        </div>
        <div class="message-bubble">${escapeHtml(msg.content)}</div>
      </div>
    `;

    const wasNearBottom = isScrollNearBottom();
    chatContainer.appendChild(wrapper);

    if (autoScroll) {
      if (wasNearBottom || isMe) {
        scrollToBottom();
      } else {
        scrollBottomBtn.classList.remove('hidden');
      }
    }
  }

  function appendSystemMessage(content, timestamp, autoScroll = true) {
    const sysEl = document.createElement('div');
    sysEl.className = 'system-event';
    sysEl.innerHTML = `<span>📢 ${escapeHtml(content)}</span> <span class="timestamp">${escapeHtml(timestamp || '')}</span>`;

    const wasNearBottom = isScrollNearBottom();
    chatContainer.appendChild(sysEl);

    if (autoScroll && wasNearBottom) {
      scrollToBottom();
    }
  }

  // Scrolling helpers
  function isScrollNearBottom() {
    const threshold = 120;
    return chatContainer.scrollHeight - chatContainer.scrollTop - chatContainer.clientHeight <= threshold;
  }

  function scrollToBottom() {
    chatContainer.scrollTop = chatContainer.scrollHeight;
    scrollBottomBtn.classList.add('hidden');
  }

  chatContainer.addEventListener('scroll', () => {
    if (isScrollNearBottom()) {
      scrollBottomBtn.classList.add('hidden');
    }
  });

  scrollBottomBtn.addEventListener('click', scrollToBottom);

  // Form Submit
  chatForm.addEventListener('submit', (e) => {
    e.preventDefault();
    const content = messageInput.value.trim();
    if (!content) return;

    const sent = sendWsMessage({
      type: 'chat',
      content: content
    });

    if (sent) {
      messageInput.value = '';
      emojiPicker.classList.add('hidden');
      messageInput.focus();
    } else {
      alert('Chưa kết nối được tới server chat. Vui lòng chờ giây lát...');
    }
  });

  // Emoji Toggle & Click
  emojiToggleBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    emojiPicker.classList.toggle('hidden');
  });

  emojiPicker.addEventListener('click', (e) => {
    if (e.target.classList.contains('emoji-item')) {
      const emoji = e.target.textContent;
      messageInput.value += emoji;
      messageInput.focus();
    }
  });

  document.addEventListener('click', (e) => {
    if (!emojiPicker.contains(e.target) && e.target !== emojiToggleBtn) {
      emojiPicker.classList.add('hidden');
    }
  });

  // Nickname Modal Handling
  function openNicknameModal() {
    nicknameInput.value = currentNickname || '';
    nicknameModal.classList.remove('hidden');
    setTimeout(() => nicknameInput.focus(), 50);
  }

  function closeNicknameModal() {
    nicknameModal.classList.add('hidden');
  }

  openNicknameModalBtn.addEventListener('click', openNicknameModal);
  closeNicknameModalBtn.addEventListener('click', closeNicknameModal);
  cancelNicknameBtn.addEventListener('click', closeNicknameModal);

  saveNicknameBtn.addEventListener('click', () => {
    const newName = nicknameInput.value.trim();
    if (newName.length < 2 || newName.length > 24) {
      alert('Biệt danh phải từ 2 đến 24 ký tự.');
      return;
    }
    if (newName === currentNickname) {
      closeNicknameModal();
      return;
    }

    sendWsMessage({
      type: 'set_nickname',
      nickname: newName
    });
    updateNicknameUI(newName);
    closeNicknameModal();
  });

  nicknameInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      saveNicknameBtn.click();
    } else if (e.key === 'Escape') {
      closeNicknameModal();
    }
  });

  // Start app
  if (currentNickname) {
    updateNicknameUI(currentNickname);
  }
  connectWebSocket();
})();
