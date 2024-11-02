import 'htmx.org';
import 'htmx-ext-sse';
import Alpine from "alpinejs"
import focus from "@alpinejs/focus";

Alpine.plugin(focus)

window.Alpine = Alpine

Alpine.start()
