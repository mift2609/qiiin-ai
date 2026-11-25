'use strict';
'require view';
'require fs';

var CTL = '/usr/bin/qiiin-ai-ctl';

function kv(s) {
	var o = {};
	(s || '').trim().split('\n').forEach(function(l) {
		var i = l.indexOf('=');
		if (i > 0) o[l.slice(0, i)] = l.slice(i + 1).trim();
	});
	return o;
}

function badge(text, ok) {
	return E('span', { 'class': 'badge ' + (ok ? 'badge-green' : 'badge-red') }, [ text ]);
}

return view.extend({
	title: _('QIIIN AI Gateway'),

	load: function() {
		return L.resolveDefault(fs.exec(CTL, [ 'status' ]), {});
	},

	status: function(r) {
		return kv(r && r.stdout);
	},

	panel: function() {
		var self = this;
		var host = E('div', { 'class': 'cbi-section' }, [ _('memuat…') ]);
		var note = E('p', { 'class': 'cbi-section-descr' }, [
			_('Gateway tetap memakai token bearer. Tombol "Buka Web AI" hanya membuka UI; token dimasukkan di halaman UI.')
		]);
		var urlLine = E('p', {}, [ _('URL: —') ]);

		var refresh = function() {
			return L.resolveDefault(fs.exec(CTL, [ 'status' ]), {}).then(function(r) {
				var s = self.status(r);
				var on = function(v) { return v === '1' || v === 'yes'; };
				host.replaceChildren(
					E('table', { 'class': 'table' }, [
						E('tr', { 'class': 'tr' }, [ E('td', { 'class': 'td' }, [ _('Model') ]), E('td', { 'class': 'td' }, [ badge(on(s.model) ? _('ada') : _('belum ada'), on(s.model)) ]) ]),
						E('tr', { 'class': 'tr' }, [ E('td', { 'class': 'td' }, [ _('Inference') ]), E('td', { 'class': 'td' }, [ badge(on(s.llama) ? _('jalan') : _('mati'), on(s.llama)) ]) ]),
						E('tr', { 'class': 'tr' }, [ E('td', { 'class': 'td' }, [ _('Gateway') ]), E('td', { 'class': 'td' }, [ badge(on(s.gateway) ? _('jalan') : _('mati'), on(s.gateway)) ]) ]),
						E('tr', { 'class': 'tr' }, [ E('td', { 'class': 'td' }, [ _('Bind') ]), E('td', { 'class': 'td' }, [ s.bind || '-' ]) ]),
						E('tr', { 'class': 'tr' }, [ E('td', { 'class': 'td' }, [ _('Context / thread / slot') ]), E('td', { 'class': 'td' }, [ (s.ctx || '-') + ' / ' + (s.threads || '-') + ' / ' + (s.parallel || '-') ]) ]),
						E('tr', { 'class': 'tr' }, [ E('td', { 'class': 'td' }, [ _('RAM tersedia') ]), E('td', { 'class': 'td' }, [ Math.round((s.mem_available_kb || 0) / 1024) + ' MB' ]) ]),
						E('tr', { 'class': 'tr' }, [ E('td', { 'class': 'td' }, [ _('Suhu SoC') ]), E('td', { 'class': 'td' }, [ (s.temp || '-') + ' °C' ]) ])
					])
				);
			});
		};

		var act = function(args, after) {
			return fs.exec(CTL, args).then(function() { return refresh(); }).then(after);
		};

		var btn = function(label, args, cls) {
			return E('button', {
				'class': 'btn cbi-button ' + (cls || 'cbi-button-apply'),
				'click': function(ev) {
					ev.preventDefault();
					return act(args);
				}
			}, [ label ]);
		};

		var openUI = function() {
			return fs.exec(CTL, [ 'url' ]).then(function(r) {
				var u = (r && r.stdout || '').trim();
				if (!u || u.indexOf('127.0.0.1') === 0) {
					alert(_('Gateway masih private (127.0.0.1). Klik "Buka akses LAN" dulu, atau akses dari HP sendiri.'));
					return;
				}
				window.open(u, '_blank');
			});
		};

		return E('div', {}, [
			host,
			E('div', { 'style': 'display:flex;gap:8px;flex-wrap:wrap;margin:12px 0' }, [
				btn(_('Start'), [ 'start' ]),
				btn(_('Stop'), [ 'stop' ], 'cbi-button-negative'),
				btn(_('Restart'), [ 'restart' ]),
				btn(_('Enable boot'), [ 'enable' ]),
				btn(_('Disable boot'), [ 'disable' ], 'cbi-button-negative')
			]),
			E('div', { 'style': 'display:flex;gap:8px;flex-wrap:wrap;margin-bottom:12px' }, [
				E('button', { 'class': 'btn cbi-button cbi-button-apply', 'click': function(ev) {
					ev.preventDefault();
					return act([ 'expose-lan' ]).then(function() { urlLine.textContent = _('URL: http://192.168.44.1:8080/'); });
				} }, [ _('Buka akses LAN') ]),
				E('button', { 'class': 'btn cbi-button', 'click': function(ev) {
					ev.preventDefault();
					return act([ 'private' ]).then(function() { urlLine.textContent = _('URL: private'); });
				} }, [ _('Kembali private') ]),
				E('button', { 'class': 'btn cbi-button cbi-button-action', 'click': function(ev) {
					ev.preventDefault();
					return openUI();
				} }, [ _('Buka Web AI') ])
			]),
			urlLine,
			note,
			E('p', { 'class': 'cbi-section-descr' }, [
				_('Membuka akses LAN hanya mengubah bind gateway menjadi 192.168.44.1:8080 (USB), bukan 0.0.0.0. Firewall tidak diubah; llama-server tetap 127.0.0.1:8081.')
			]),
			E('button', { 'class': 'btn cbi-button', 'click': function(ev) { ev.preventDefault(); return refresh(); } }, [ _('Refresh') ])
		]);
	},

	render: function(data) {
		return E('div', {}, [ this.panel(), E('p', {}, [ _('diperbarui ') + new Date().toLocaleTimeString() ]) ]);
	}
});
