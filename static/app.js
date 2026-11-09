// Rust Global Chat - Multi-Room & Media Client
(() => {
  // Elements
  const statusDot = document.getElementById('statusDot');
  const statusText = document.getElementById('statusText');
  const userCountEl = document.getElementById('userCount');
  const myNicknameEl = document.getElementById('myNickname');
  const myAvatarEl = document.getElementById('myAvatar');
  const chatContainer = document.getElementById('chatContainer');
  const chatForm = document.getElementById('chatForm');
  const messageInput = document.getElementById('messageInput');
  const scrollBottomBtn = document.getElementById('scrollBottomBtn');
  const emojiToggleBtn = document.getElementById('emojiToggleBtn');
  const emojiPicker = document.getElementById('emojiPicker');
  const gifToggleBtn = document.getElementById('gifToggleBtn');
  const gifPicker = document.getElementById('gifPicker');
  const gifPickerClose = document.getElementById('gifPickerClose');
  const gifSearchInput = document.getElementById('gifSearchInput');
  const gifQuickTags = document.getElementById('gifQuickTags');
  const gifResults = document.getElementById('gifResults');

  // Channel elements
  const channelsList = document.getElementById('channelsList');
  const currentChannelTitle = document.getElementById('currentChannelTitle');
  const currentChannelTopic = document.getElementById('currentChannelTopic');
  const welcomeTitle = document.getElementById('welcomeTitle');
  const welcomeDesc = document.getElementById('welcomeDesc');
  const appSidebar = document.getElementById('appSidebar');
  const sidebarToggleBtn = document.getElementById('sidebarToggleBtn');
  const sidebarCloseBtn = document.getElementById('sidebarCloseBtn');

  // Modal elements
  const openNicknameModalBtn = document.getElementById('openNicknameModalBtn');
  const nicknameModal = document.getElementById('nicknameModal');
  const closeNicknameModalBtn = document.getElementById('closeNicknameModalBtn');
  const cancelNicknameBtn = document.getElementById('cancelNicknameBtn');
  const saveNicknameBtn = document.getElementById('saveNicknameBtn');
  const nicknameInput = document.getElementById('nicknameInput');

  // Lightbox elements
  const lightboxModal = document.getElementById('lightboxModal');
  const lightboxImage = document.getElementById('lightboxImage');
  const lightboxCloseBtn = document.getElementById('lightboxCloseBtn');

  // State
  let ws = null;
  let currentRoom = (window.location.hash || '').replace('#', '').trim() || 'general';
  let currentNickname = localStorage.getItem('rust_chat_nickname') || '';
  let reconnectTimeout = null;
  let reconnectAttempts = 0;
  const maxReconnectDelay = 10000;

  const roomTopics = {
    'general': 'Phòng trò chuyện sảnh chính',
    'rust-lang': 'Góc giao lưu kỹ thuật & dự án bằng Rust',
    'gaming': 'Kênh chém gió game thủ, rủ chơi game',
    'chill': 'Góc thư giãn, nghe nhạc và tán gẫu'
  };

  // Curated High-Quality GIFs Fallback / Fast Loading
  const curatedGifs = {
    'trending': [
      'https://media.tenor.com/2lFh_d9rV9QAAAAC/cat-vibing.gif',
      'https://media.tenor.com/8N4J96n3RuoAAAAC/pepe-dance.gif',
      'https://media.tenor.com/qLhP75vU3vIAAAAC/popcat.gif',
      'https://media.tenor.com/PZtN_y3U3UAAAAAC/leonardo-dicaprio-cheers.gif',
      'https://media.tenor.com/O61g7U029VAAAAAC/mind-blown.gif',
      'https://media.tenor.com/kHdt_22T_AAAAAAC/dancing-happy.gif'
    ],
    'rust crab': [
      'https://media.tenor.com/mO24nS23G0AAAAAC/crab-rave-dancing.gif',
      'https://media.tenor.com/w162i0sWlV4AAAAC/ferris-rust.gif',
      'https://media.tenor.com/M6L54yZ0u7AAAAAC/crab-rave.gif',
      'https://media.tenor.com/yS42JvE_s7EAAAAC/sponge-bob-mr-krabs.gif'
    ],
    'laugh': [
      'https://media.tenor.com/bC8w56S_W_0AAAAC/laughing-meme.gif',
      'https://media.tenor.com/4i9JgW_Z40sAAAAC/lol-laughing.gif',
      'https://media.tenor.com/0F4vN3ZJmXoAAAAC/crying-laughing.gif',
      'https://media.tenor.com/mU0H9l1_J-QAAAAC/hahaha.gif'
    ],
    'cat': [
      'https://media.tenor.com/qLhP75vU3vIAAAAC/popcat.gif',
      'https://media.tenor.com/2lFh_d9rV9QAAAAC/cat-vibing.gif',
      'https://media.tenor.com/fT_SjKq8bPAAAAAC/cat-jam.gif',
      'https://media.tenor.com/y_WvjGZ2L2kAAAAC/shocked-cat.gif'
    ],
    'dance': [
      'https://media.tenor.com/8N4J96n3RuoAAAAC/pepe-dance.gif',
      'https://media.tenor.com/mO24nS23G0AAAAAC/crab-rave-dancing.gif',
      'https://media.tenor.com/kHdt_22T_AAAAAAC/dancing-happy.gif',
      'https://media.tenor.com/OqD0nS_n9XwAAAAC/snoopy-dance.gif'
    ],
    'wow': [
      'https://media.tenor.com/O61g7U029VAAAAAC/mind-blown.gif',
      'https://media.tenor.com/f_G2f4N1j7QAAAAC/omg-wow.gif',
      'https://media.tenor.com/f9h_s1_jK9sAAAAC/shocked.gif',
      'https://media.tenor.com/Yv7Xk7_r4-oAAAAC/keanu-reeves-whoa.gif'
    ]
  };

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

  // Detect image and GIF URLs
  function extractMediaUrl(text) {
    const mediaPattern = /(https?:\/\/[^\s]+(?:\.png|\.jpg|\.jpeg|\.gif|\.webp)(?:\?[^\s]*)?|https?:\/\/(?:media\.tenor\.com|media[0-9]?\.giphy\.com|i\.imgur\.com|i\.redd\.it)[^\s]+)/i;
    const match = text.match(mediaPattern);
    if (match) {
      const url = match[0];
      const textWithoutUrl = text.replace(url, '').trim();
      return { url, text: textWithoutUrl };
    }
    return null;
  }

  // Room switching logic
  function switchRoom(roomName) {
    if (!roomName) roomName = 'general';
    currentRoom = roomName;
    window.location.hash = roomName;

    // Update UI highlights
    document.querySelectorAll('.channel-item').forEach(btn => {
      btn.classList.toggle('active', btn.dataset.room === roomName);
    });

    currentChannelTitle.textContent = roomName;
    currentChannelTopic.textContent = roomTopics[roomName] || `Phòng #${roomName}`;
    welcomeTitle.textContent = `Chào mừng bạn đến với #${roomName}!`;
    welcomeDesc.textContent = `Đây là điểm bắt đầu của phòng trò chuyện #${roomName}.`;
    messageInput.placeholder = `Nhắn tin trong #${roomName}... (hỗ trợ dán link ảnh/GIF)`;

    // Close mobile sidebar
    appSidebar.classList.remove('open');

    // Notify WebSocket server
    sendWsMessage({
      type: 'join_room',
      room: roomName
    });
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

      if (currentNickname) {
        sendWsMessage({
          type: 'set_nickname',
          nickname: currentNickname
        });
      }

      if (currentRoom && currentRoom !== 'general') {
        sendWsMessage({
          type: 'join_room',
          room: currentRoom
        });
      }
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        handleServerMessage(data);
      } catch (err) {
        console.error('Lỗi parse tin nhắn:', err);
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

  // Handle server incoming messages
  function handleServerMessage(msg) {
    switch (msg.type) {
      case 'nickname_assigned':
        updateNicknameUI(msg.nickname);
        break;

      case 'user_count':
        userCountEl.textContent = msg.count;
        break;

      case 'room_joined':
        if (msg.room !== currentRoom) {
          switchRoom(msg.room);
        }
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
        console.warn('Loại tin không xác định:', msg);
    }
  }

  function renderHistory(messages) {
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

    const mediaInfo = extractMediaUrl(msg.content);
    let bubbleContentHtml = '';

    if (mediaInfo) {
      if (mediaInfo.text) {
        bubbleContentHtml += `<div class="bubble-text">${escapeHtml(mediaInfo.text)}</div>`;
      }
      bubbleContentHtml += `
        <div class="chat-media-container" data-img-url="${escapeHtml(mediaInfo.url)}">
          <img src="${escapeHtml(mediaInfo.url)}" class="chat-media-img" loading="lazy" alt="Media image" onerror="this.parentElement.style.display='none'"/>
        </div>
      `;
    } else {
      bubbleContentHtml = escapeHtml(msg.content);
    }

    wrapper.innerHTML = `
      <div class="avatar" style="background: ${color}">${initials}</div>
      <div class="message-content-group">
        <div class="message-header">
          <span class="sender-name">${escapeHtml(msg.nickname)}</span>
          <span class="timestamp">${escapeHtml(msg.timestamp || '')}</span>
        </div>
        <div class="message-bubble">${bubbleContentHtml}</div>
      </div>
    `;

    // Click to open lightbox
    const mediaContainer = wrapper.querySelector('.chat-media-container');
    if (mediaContainer) {
      mediaContainer.addEventListener('click', () => {
        openLightbox(mediaContainer.dataset.imgUrl);
      });
    }

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
    const isWarning = content.includes('⚠️');
    const sysEl = document.createElement('div');
    sysEl.className = `system-event ${isWarning ? 'warning' : ''}`;
    const icon = isWarning ? '🚨' : '📢';
    sysEl.innerHTML = `<span>${icon} ${escapeHtml(content)}</span> <span class="timestamp">${escapeHtml(timestamp || '')}</span>`;

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

    const sendBtn = document.getElementById('sendBtn');
    if (sendBtn) {
      sendBtn.disabled = true;
      setTimeout(() => { sendBtn.disabled = false; }, 300);
    }

    const sent = sendWsMessage({
      type: 'chat',
      content: content
    });

    if (sent) {
      messageInput.value = '';
      emojiPicker.classList.add('hidden');
      gifPicker.classList.add('hidden');
      messageInput.focus();
    } else {
      alert('Chưa kết nối được tới server chat. Vui lòng thử lại...');
    }
  });

  // Channels click listener
  channelsList.addEventListener('click', (e) => {
    const btn = e.target.closest('.channel-item');
    if (btn && btn.dataset.room) {
      switchRoom(btn.dataset.room);
    }
  });

  // Mobile sidebar toggle
  sidebarToggleBtn.addEventListener('click', () => {
    appSidebar.classList.add('open');
  });

  sidebarCloseBtn.addEventListener('click', () => {
    appSidebar.classList.remove('open');
  });

  // Emoji Toggle
  emojiToggleBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    gifPicker.classList.add('hidden');
    emojiPicker.classList.toggle('hidden');
  });

  emojiPicker.addEventListener('click', (e) => {
    if (e.target.classList.contains('emoji-item')) {
      messageInput.value += e.target.textContent;
      messageInput.focus();
    }
  });

  // GIF Picker Handling
  gifToggleBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    emojiPicker.classList.add('hidden');
    gifPicker.classList.toggle('hidden');
    if (!gifPicker.classList.contains('hidden')) {
      loadGifsForTag('trending');
    }
  });

  gifPickerClose.addEventListener('click', () => {
    gifPicker.classList.add('hidden');
  });

  gifQuickTags.addEventListener('click', (e) => {
    const tag = e.target.closest('.gif-tag');
    if (tag && tag.dataset.tag) {
      loadGifsForTag(tag.dataset.tag);
    }
  });

  function renderGifList(urls) {
    gifResults.innerHTML = '';
    urls.forEach(url => {
      const img = document.createElement('img');
      img.src = url;
      img.className = 'gif-item';
      img.loading = 'lazy';
      img.addEventListener('click', () => {
        // Send GIF URL directly
        sendWsMessage({
          type: 'chat',
          content: url
        });
        gifPicker.classList.add('hidden');
      });
      gifResults.appendChild(img);
    });
  }

  async function loadGifsForTag(tag) {
    // If we have curated, render immediately
    if (curatedGifs[tag]) {
      renderGifList(curatedGifs[tag]);
      return;
    }

    gifResults.innerHTML = '<div style="color:var(--text-muted);font-size:0.8rem;padding:1rem;">Đang tìm kiếm...</div>';

    try {
      // Tenor API search
      const res = await fetch(`https://tenor.googleapis.com/v2/search?q=${encodeURIComponent(tag)}&key=LIVDSRZULELA&limit=12&media_filter=gif`);
      if (res.ok) {
        const data = await res.json();
        if (data.results && data.results.length > 0) {
          const urls = data.results.map(item => item.media_formats.gif.url);
          renderGifList(urls);
          return;
        }
      }
    } catch (e) {
      console.warn('Tenor API rate limit or error, using fallback:', e);
    }

    // Fallback to trending
    renderGifList(curatedGifs['trending']);
  }

  gifSearchInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      const query = gifSearchInput.value.trim();
      if (query) {
        loadGifsForTag(query);
      }
    }
  });

  // Close popovers on outer click
  document.addEventListener('click', (e) => {
    if (!emojiPicker.contains(e.target) && e.target !== emojiToggleBtn) {
      emojiPicker.classList.add('hidden');
    }
    if (!gifPicker.contains(e.target) && e.target !== gifToggleBtn) {
      gifPicker.classList.add('hidden');
    }
  });

  // Lightbox Modal
  function openLightbox(url) {
    lightboxImage.src = url;
    lightboxModal.classList.remove('hidden');
  }

  function closeLightbox() {
    lightboxModal.classList.add('hidden');
    lightboxImage.src = '';
  }

  lightboxCloseBtn.addEventListener('click', closeLightbox);
  lightboxModal.addEventListener('click', (e) => {
    if (e.target === lightboxModal) {
      closeLightbox();
    }
  });

  // Nickname Modal
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

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      closeLightbox();
      closeNicknameModal();
    }
  });

  // Start app
  if (currentNickname) {
    updateNicknameUI(currentNickname);
  }
  switchRoom(currentRoom);
  connectWebSocket();
})();
