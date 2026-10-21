#include <cJSON.h>
#include <stdio.h>
#include <stdlib.h>

int main(void)
{
    cJSON *j = cJSON_Parse("{\"a\":[1,2,{\"b\":null}]}");
    char *s = j ? cJSON_PrintUnformatted(j) : NULL;
    int ok = s && puts(s) >= 0;
    free(s);
    cJSON_Delete(j);
    return ok ? 0 : 1;
}
