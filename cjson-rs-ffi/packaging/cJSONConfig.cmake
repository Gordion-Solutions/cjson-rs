# Relocatable CMake package for cjson-rs, shaped like cJSON's own:
#   find_package(cJSON)  ->  CJSON_INCLUDE_DIRS, CJSON_LIBRARIES, target cJSON::cjson
get_filename_component(_cjson_prefix "${CMAKE_CURRENT_LIST_DIR}/../../.." ABSOLUTE)
set(CJSON_INCLUDE_DIRS "${_cjson_prefix}/include/cjson")
if(APPLE)
  set(_cjson_lib "${_cjson_prefix}/lib/libcjson.dylib")
else()
  set(_cjson_lib "${_cjson_prefix}/lib/libcjson.so")
endif()
set(CJSON_LIBRARY "${_cjson_lib}")
set(CJSON_LIBRARIES "${_cjson_lib}")
if(NOT TARGET cJSON::cjson)
  add_library(cJSON::cjson SHARED IMPORTED)
  set_target_properties(cJSON::cjson PROPERTIES
    IMPORTED_LOCATION "${_cjson_lib}"
    INTERFACE_INCLUDE_DIRECTORIES "${CJSON_INCLUDE_DIRS}")
  if(NOT APPLE)
    set_target_properties(cJSON::cjson PROPERTIES IMPORTED_SONAME "libcjson.so.1")
  endif()
endif()

# cJSON_Utils (JSON Pointer/Patch/Merge Patch). The symbols live in the same
# library; libcjson_utils is a link to it, matching cJSON's own package which
# exposes a cJSON::cjson_utils target.
if(APPLE)
  set(_cjson_utils_lib "${_cjson_prefix}/lib/libcjson_utils.dylib")
else()
  set(_cjson_utils_lib "${_cjson_prefix}/lib/libcjson_utils.so")
endif()
set(CJSON_UTILS_LIBRARY "${_cjson_utils_lib}")
set(CJSON_UTILS_LIBRARIES "${_cjson_utils_lib}")
if(NOT TARGET cJSON::cjson_utils)
  add_library(cJSON::cjson_utils SHARED IMPORTED)
  set_target_properties(cJSON::cjson_utils PROPERTIES
    IMPORTED_LOCATION "${_cjson_utils_lib}"
    INTERFACE_INCLUDE_DIRECTORIES "${CJSON_INCLUDE_DIRS}"
    INTERFACE_LINK_LIBRARIES cJSON::cjson)
  if(NOT APPLE)
    set_target_properties(cJSON::cjson_utils PROPERTIES IMPORTED_SONAME "libcjson.so.1")
  endif()
endif()
set(cJSON_FOUND TRUE)
set(CJSON_FOUND TRUE)
unset(_cjson_prefix)
unset(_cjson_lib)
unset(_cjson_utils_lib)
