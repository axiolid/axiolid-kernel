include(FetchContent)

function(axiolid_fetch)
  set(one_value GIT_REPOSITORY GIT_COMMIT LINKAGE)
  cmake_parse_arguments(AXIOLID "" "${one_value}" "" ${ARGN})
  if(NOT AXIOLID_GIT_REPOSITORY)
    set(AXIOLID_GIT_REPOSITORY "https://github.com/axiolid/kernel.git")
  endif()
  string(LENGTH "${AXIOLID_GIT_COMMIT}" _commit_length)
  if(NOT _commit_length EQUAL 40 OR NOT AXIOLID_GIT_COMMIT MATCHES "^[0-9a-fA-F]+$")
    message(FATAL_ERROR "axiolid_fetch requires an immutable 40-hex GIT_COMMIT; branches and movable tags are refused")
  endif()
  if(AXIOLID_LINKAGE)
    set(AXIOLID_LINKAGE "${AXIOLID_LINKAGE}" CACHE STRING "Axiolid linkage" FORCE)
  endif()
  FetchContent_Declare(axiolid_source
    GIT_REPOSITORY "${AXIOLID_GIT_REPOSITORY}"
    GIT_TAG "${AXIOLID_GIT_COMMIT}"
    GIT_SHALLOW FALSE
    GIT_SUBMODULES ""
    SOURCE_SUBDIR native
  )
  FetchContent_MakeAvailable(axiolid_source)
endfunction()
