// Fork: manual "Check for updates"; a newer release shows the existing update card on Home.
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:flutter_hbb/models/state_model.dart';

Future<void> checkForUpdates() async {
  final url = await bind.mainGetCommon(key: 'fork-check-update');
  if (url.startsWith('error:')) {
    showToast(url.substring('error:'.length));
    return;
  }
  stateGlobal.updateUrl.value = url;
  showToast(url.isEmpty
      ? translate('You are using the latest version')
      : '${translate('New version available')}: ${url.split('/').last}');
}

/// Release tags pad the patch to 3 digits (2.0.1 -> 2.0.001); show the version the same way.
String forkDisplayVersion(String v) =>
    v.replaceFirstMapped(RegExp(r'\.(\d{1,2})$'), (m) => '.${m[1]!.padLeft(3, '0')}');

/// Release assets keep the plain number (2.0.001 -> 2.0.1).
String forkFileVersion(String tag) => tag.replaceFirst(RegExp(r'\.0+(?=\d+$)'), '.');
