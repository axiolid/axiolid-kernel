cmake_minimum_required(VERSION 3.24)
project(AxiolidCMakeConsumer LANGUAGES C CXX)

if(AXIOLID_SOURCE_TREE)
  add_subdirectory("${AXIOLID_SOURCE_TREE}/native" axiolid-native)
else()
  find_package(Axiolid 0.4 CONFIG REQUIRED)
endif()

add_executable(axiolid-cmake-consumer main.c)
add_executable(axiolid-cmake-consumer-cpp main.cpp)
foreach(consumer IN ITEMS axiolid-cmake-consumer axiolid-cmake-consumer-cpp)
  target_link_libraries(${consumer} PRIVATE Axiolid::axiolid)
  if(WIN32 AND AXIOLID_LINKAGE STREQUAL "SHARED")
    add_custom_command(TARGET ${consumer} POST_BUILD
      COMMAND "${CMAKE_COMMAND}" -E copy_if_different
              "$<TARGET_FILE:Axiolid::axiolid_shared>"
              "$<TARGET_FILE_DIR:${consumer}>"
    )
  endif()
endforeach()

enable_testing()
add_test(NAME axiolid-cmake-consumer COMMAND axiolid-cmake-consumer)
add_test(NAME axiolid-cmake-consumer-cpp COMMAND axiolid-cmake-consumer-cpp)
