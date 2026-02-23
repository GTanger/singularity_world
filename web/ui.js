// 富文字區、動作選單；對齊決策 004 HTML 原生富文字處理詞盤、對話、敘事。
(function () {
	function updateActions(buttons) {
		const el = document.getElementById('actions');
		if (!el) return;
		el.innerHTML = '';
		(buttons || []).forEach(function (label) {
			const btn = document.createElement('button');
			btn.textContent = label;
			el.appendChild(btn);
		});
	}

	window.uiUpdateActions = updateActions;
})();
