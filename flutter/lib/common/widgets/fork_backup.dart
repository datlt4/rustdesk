// Fork: export / import peers (with saved passwords), settings and server profiles as one JSON file.
import 'dart:convert';
import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/models/platform_model.dart';

Future<void> exportBackup() async {
  final json = await bind.mainGetCommon(key: 'fork-export-backup');
  if (json.startsWith('error:')) {
    showToast(json.substring('error:'.length));
    return;
  }
  final date = DateTime.now().toIso8601String().substring(0, 10);
  final name = 'rustdesk-backup-$date.json';
  if (isDesktop) {
    final path = await FilePicker.platform.saveFile(
        fileName: name, type: FileType.custom, allowedExtensions: ['json']);
    if (path == null) return;
    await File(path).writeAsString(json);
    showToast(translate('Successful'));
    return;
  }
  // file_picker 5 cannot save on phones: Android writes to Download, otherwise the clipboard.
  if (isAndroid) {
    try {
      final file = File('/storage/emulated/0/Download/$name');
      await file.writeAsString(json);
      showToast(file.path);
      return;
    } catch (e) {
      debugPrint('Backup to Download failed: $e');
    }
  }
  await Clipboard.setData(ClipboardData(text: json));
  showToast(translate('Copied'));
}

Future<void> importBackup() async {
  final FilePickerResult? picked;
  try {
    picked =
        await FilePicker.platform.pickFiles(type: FileType.any, withData: true);
  } catch (e) {
    showToast('$e');
    return;
  }
  final bytes = picked?.files.single.bytes;
  if (bytes == null) return;
  final res = await bind.mainGetCommon(
      key: 'fork-import-backup:${utf8.decode(bytes, allowMalformed: true)}');
  if (res.startsWith('error:')) {
    showToast(res.substring('error:'.length));
    return;
  }
  bind.mainLoadRecentPeers();
  bind.mainLoadFavPeers();
  showToast(translate('Successful'));
}
