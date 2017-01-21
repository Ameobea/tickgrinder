package tickrecorder;

public interface PublicConfig extends PrivateConfig {
    public String[] monitoredPairs = {"USD/CAD", "EUR/USD", "EUR/JPY", "AUD/USD"};
    public String connectionType = "Demo";
}
