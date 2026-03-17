<?php
/**
 * RustyLox PHP Bootstrap
 *
 * Compatibility layer for running LoxBerry plugins under PHP 8.x
 * Defines constants and shims that older plugins expect.
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
