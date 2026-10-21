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
set(cJSON_FOUND TRUE)
set(CJSON_FOUND TRUE)
unset(_cjson_prefix)
unset(_cjson_lib)
