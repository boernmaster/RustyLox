<?php
/**
 * RustyLox PHP Bootstrap
 *
 * Compatibility layer for running LoxBerry plugins under PHP 8.x CLI
 * with CGI environment variables. Populates $_GET, $_POST, $_SERVER
 * from environment, and defines constants for PHP 8 compatibility.
 */

// Common constants used as array keys in LoxBerry plugins
// In PHP 7.x these would silently be treated as strings; PHP 8.x throws fatal errors
if (!defined('FORMID'))    define('FORMID',    'FORMID');
if (!defined('DATA_MINI')) define('DATA_MINI', 'DATA_MINI');
if (!defined('LABEL'))     define('LABEL',     'LABEL');
if (!defined('SELECTED'))  define('SELECTED',  'SELECTED');
if (!defined('ENABLED'))   define('ENABLED',   'ENABLED');
if (!defined('DISABLED'))  define('DISABLED',  'DISABLED');
if (!defined('READONLY'))  define('READONLY',  'READONLY');
if (!defined('NAME'))      define('NAME',      'NAME');
if (!defined('VALUE'))     define('VALUE',     'VALUE');
if (!defined('TYPE'))      define('TYPE',      'TYPE');

// Suppress non-critical warnings that may appear from older PHP code
error_reporting(E_ERROR | E_PARSE);

// When running under php-cgi, $_SERVER, $_GET, $_POST are populated automatically.
// Only populate them manually when running under php-cli (no CGI SAPI).
if (php_sapi_name() !== 'cgi-fcgi' && php_sapi_name() !== 'cgi') {
    // Populate $_SERVER from CGI environment variables
    $_SERVER['REQUEST_METHOD'] = getenv('REQUEST_METHOD') ?: 'GET';
    $_SERVER['QUERY_STRING'] = getenv('QUERY_STRING') ?: '';
    $_SERVER['CONTENT_TYPE'] = getenv('CONTENT_TYPE') ?: '';
    $_SERVER['CONTENT_LENGTH'] = getenv('CONTENT_LENGTH') ?: '0';
    $_SERVER['SCRIPT_FILENAME'] = getenv('SCRIPT_FILENAME') ?: '';
    $_SERVER['SERVER_PROTOCOL'] = getenv('SERVER_PROTOCOL') ?: 'HTTP/1.1';
    $_SERVER['GATEWAY_INTERFACE'] = getenv('GATEWAY_INTERFACE') ?: 'CGI/1.1';
    $_SERVER['SERVER_SOFTWARE'] = getenv('SERVER_SOFTWARE') ?: 'RustyLox';
    $_SERVER['REDIRECT_STATUS'] = getenv('REDIRECT_STATUS') ?: '200';

    // Populate $_GET from QUERY_STRING
    if (!empty($_SERVER['QUERY_STRING'])) {
        parse_str($_SERVER['QUERY_STRING'], $_GET);
    }

    // Populate $_POST from stdin for POST requests
    if ($_SERVER['REQUEST_METHOD'] === 'POST') {
        $content_type = $_SERVER['CONTENT_TYPE'];
        $content_length = (int)$_SERVER['CONTENT_LENGTH'];

        if ($content_length > 0) {
            $raw_post = file_get_contents('php://stdin');

            if (strpos($content_type, 'application/x-www-form-urlencoded') !== false) {
                parse_str($raw_post, $_POST);
            } elseif (strpos($content_type, 'application/json') !== false) {
                $_POST = json_decode($raw_post, true) ?: [];
            }

            // Also make raw POST data available
            $GLOBALS['HTTP_RAW_POST_DATA'] = $raw_post;
        }
    }

    // Populate $_REQUEST (combination of $_GET and $_POST)
    $_REQUEST = array_merge($_GET, $_POST);
}

// Ensure SERVER_PROTOCOL is set (used by plugin sendresponse functions)
if (empty($_SERVER['SERVER_PROTOCOL'])) {
    $_SERVER['SERVER_PROTOCOL'] = 'HTTP/1.1';
}

// Fix get_included_files() for auto_prepend_file usage.
// loxberry_system.php uses get_included_files()[0] to determine the plugin name
// from the script path. With auto_prepend_file, this bootstrap becomes [0] instead
// of the actual script. We override the internal list by including the real script
// path info via a global that loxberry_system.php can check first.
if (getenv('SCRIPT_FILENAME')) {
    $GLOBALS['_LOXBERRY_SCRIPT_PATH'] = getenv('SCRIPT_FILENAME');
}
