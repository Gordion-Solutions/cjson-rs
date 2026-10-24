#include <cJSON.h>
#include <cJSON_Utils.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void)
{
    cJSON *j = cJSON_Parse("{\"a\":[1,2,{\"b\":null}]}");
    char *s = j ? cJSON_PrintUnformatted(j) : NULL;
    int ok = s && puts(s) >= 0;
    free(s);
    cJSON_Delete(j);

    /* Exercise cJSON_Utils too, so the install's libcjson_utils is proven. */
    cJSON *doc = cJSON_Parse("{\"x\":{\"y\":1}}");
    const cJSON *at = cJSONUtils_GetPointer(doc, "/x/y");
    cJSON *patch = cJSON_Parse("{\"x\":{\"z\":2}}");
    doc = cJSONUtils_MergePatch(doc, patch);
    char *merged = doc ? cJSON_PrintUnformatted(doc) : NULL;
    ok = ok && at && merged && strcmp(merged, "{\"x\":{\"y\":1,\"z\":2}}") == 0;
    free(merged);
    cJSON_Delete(doc);
    cJSON_Delete(patch);

    return ok ? 0 : 1;
}
