package sequent.keycloak.inetum_authenticator;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.sql.Statement;

public class ImmuDbConnectionService  {

    private static final String DB_URL = System.getenv("IMMUDB_SERVER_URL");
    private static final String USER = System.getenv("IMMUDDB_USER");
    private static final String PASS = System.getenv("IMMUDDB_PASS");
    private static Connection conn = null;
    private static Statement stmt = null;

    public ImmuDbConnectionService() {
        try {
            conn = DriverManager.getConnection(DB_URL, USER, PASS);
            stmt = conn.createStatement();
        } catch (Exception e) {

        }
    }

    public void insert(String realmName, String value) {
        try {
            String dbName = getDbName(realmName);
            String sql = "INSERT INTO immudb (key, value) VALUES ('"  + "', '" + value + "')";
            stmt.executeUpdate(sql);
        } catch (SQLException e) {

        }
    }

    private String getDbName(String realmName) {
        return realmName.replace("-", "");
    }

    public void closeConnection() {
        try {
            stmt.close();
            conn.close();
        } catch (Exception e) {

        }
    }
}
