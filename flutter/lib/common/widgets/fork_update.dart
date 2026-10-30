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
